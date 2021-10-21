package parachain

import (
	"context"
	"encoding/hex"
	"fmt"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/snowfork/snowbridge/relayer/chain/relaychain"
	"github.com/snowfork/snowbridge/relayer/contracts/basic"
	"github.com/snowfork/snowbridge/relayer/contracts/incentivized"
	"github.com/snowfork/snowbridge/relayer/crypto/keccak"
	"github.com/vovac12/go-substrate-rpc-client/v3/types"

	log "github.com/sirupsen/logrus"
)

// Catches up by searching for and relaying all missed commitments before the given para block
// This method creates proofs based on the mmr root at the specific given relaychainBlock and so
// the proofs will need to be verified by the mmr root for that relay chain block
func (li *BeefyListener) buildMissedMessagePackages(
	ctx context.Context, relaychainBlock uint64, relaychainBlockHash types.Hash) (
	[]MessagePackage, error) {
	basicContract, err := basic.NewBasicInboundChannel(common.HexToAddress(
		li.config.Contracts.BasicInboundChannel),
		li.ethereumConn.GetClient(),
	)
	if err != nil {
		return nil, err
	}

	incentivizedContract, err := incentivized.NewIncentivizedInboundChannel(common.HexToAddress(
		li.config.Contracts.IncentivizedInboundChannel),
		li.ethereumConn.GetClient(),
	)
	if err != nil {
		return nil, err
	}

	options := bind.CallOpts{
		Pending: true,
		Context: ctx,
	}

	ethBasicNonce, err := basicContract.Nonce(&options)
	if err != nil {
		return nil, err
	}
	log.WithFields(log.Fields{
		"nonce": ethBasicNonce,
	}).Info("Checked latest nonce delivered to ethereum basic channel")

	ethIncentivizedNonce, err := incentivizedContract.Nonce(&options)
	if err != nil {
		return nil, err
	}
	log.WithFields(log.Fields{
		"nonce": ethIncentivizedNonce,
	}).Info("Checked latest nonce delivered to ethereum incentivized channel")

	paraBasicNonceKey, err := types.CreateStorageKey(li.relaychainConn.Metadata(), "BasicOutboundChannel", "Nonce", nil, nil)
	if err != nil {
		log.Error(err)
		return nil, err
	}
	var paraBasicNonce types.U64
	ok, err := li.relaychainConn.API().RPC.State.GetStorage(paraBasicNonceKey, &paraBasicNonce, relaychainBlockHash)
	if err != nil {
		log.Error(err)
		return nil, err
	}
	if !ok {
		paraBasicNonce = 0
	}
	log.WithFields(log.Fields{
		"nonce": uint64(paraBasicNonce),
	}).Info("Checked latest nonce generated by parachain basic channel")

	paraIncentivizedNonceKey, err := types.CreateStorageKey(li.relaychainConn.Metadata(), "IncentivizedOutboundChannel", "Nonce", nil, nil)
	if err != nil {
		log.Error(err)
		return nil, err
	}
	var paraIncentivizedNonce types.U64
	ok, err = li.relaychainConn.API().RPC.State.GetStorage(paraIncentivizedNonceKey, &paraIncentivizedNonce, relaychainBlockHash)
	if err != nil {
		log.Error(err)
		return nil, err
	}
	if !ok {
		paraIncentivizedNonce = 0
	}
	log.WithFields(log.Fields{
		"nonce": uint64(paraIncentivizedNonce),
	}).Info("Checked latest nonce generated by parachain incentivized channel")

	if ethBasicNonce == uint64(paraBasicNonce) && ethIncentivizedNonce == uint64(paraIncentivizedNonce) {
		return nil, nil
	}

	log.Info("Nonces are not all up to date - searching for lost commitments")

	paraBlocks, err := li.searchForLostCommitments(relaychainBlock, ethBasicNonce, ethIncentivizedNonce)
	if err != nil {
		return nil, err
	}

	log.Info("Stopped searching for lost commitments")

	log.WithFields(log.Fields{
		"blocks": paraBlocks,
	}).Info("Found these blocks and commitments")

	blocksWithProofs, err := li.parablocksWithProofs(paraBlocks, relaychainBlock, relaychainBlockHash)
	if err != nil {
		log.Error(err)
		return nil, err
	}

	// Reverse blocks to be in ascending order
	for i, j := 0, len(blocksWithProofs)-1; i < j; i, j = i+1, j-1 {
		blocksWithProofs[i], blocksWithProofs[j] = blocksWithProofs[j], blocksWithProofs[i]
	}

	log.Info("Packaging blocks and proofs")

	mmrLeafCount, err := li.relaychainConn.FetchMMRLeafCount(relaychainBlockHash)
	if err != nil {
		log.WithError(err).Error("Failed get MMR Leaf Count")
		return nil, err
	}

	messagePackages, err := CreateMessagePackages(blocksWithProofs, mmrLeafCount)
	if err != nil {
		log.WithError(err).Error("Failed to create message packages")
		return nil, err
	}

	log.WithField("messages", messagePackages).Info("Created message packages")

	return messagePackages, nil
}

// Takes a slice of parachain blocks and augments them with their respective
// header, header proof and MMR proof at the given relay chain block mmr root
func (li *BeefyListener) parablocksWithProofs(
	blocks []ParaBlockWithDigest,
	latestRelayChainBlockNumber uint64,
	latestRelaychainBlockHash types.Hash,
) ([]ParaBlockWithProofs, error) {
	relayChainBlockNumber := latestRelayChainBlockNumber
	var blocksWithProof []ParaBlockWithProofs
	for _, block := range blocks {
		// Note - relayChainBlockNumber will be one less than the actual block number we want
		// due to the decrement at the end of the loop, so we increment by 1. Additionally,
		// parachain merkle roots are created 1 block later than the actual parachain headers,
		// so we increment twice.

		log.WithFields(log.Fields{
			"blockNumber": relayChainBlockNumber,
			"blockHash":   latestRelaychainBlockHash.Hex(),
		}).Info("Getting MMR Leaf for block...")

		mmrProof, err := li.relaychainConn.GetMMRLeafForBlock(block.BlockNumber+1, latestRelaychainBlockHash, li.config.Substrate.BeefyStartingBlock)
		if err != nil {
			log.WithError(err).Error("Failed to get mmr leaf")
			return nil, err
		}

		mmrRootHashKey, err := types.CreateStorageKey(li.relaychainConn.Metadata(), "Mmr", "RootHash", nil, nil)
		if err != nil {
			log.Error(err)
			return nil, err
		}
		var mmrRootHash types.Hash
		ok, err := li.relaychainConn.API().RPC.State.GetStorage(mmrRootHashKey, &mmrRootHash, latestRelaychainBlockHash)
		if err != nil {
			log.Error(err)
			return nil, err
		}
		if !ok {
			return nil, fmt.Errorf("could not get mmr root hash")
		}

		blockWithProof := ParaBlockWithProofs{
			Block:             block,
			MMRProofResponse:  mmrProof,
			MMRRootHash:       mmrRootHash,
			mmrProofLeafIndex: latestRelayChainBlockNumber - 1,
		}
		blocksWithProof = append(blocksWithProof, blockWithProof)
	}
	return blocksWithProof, nil
}

// Searches for all lost commitments on each channel from the given parachain block number backwards
// until it finds the given basic and incentivized nonce
func (li *BeefyListener) searchForLostCommitments(
	lastParaBlockNumber uint64,
	basicNonceToFind uint64,
	incentivizedNonceToFind uint64) ([]ParaBlockWithDigest, error) {
	log.WithFields(log.Fields{
		"basicNonce":        basicNonceToFind,
		"incentivizedNonce": incentivizedNonceToFind,
		"latestblockNumber": lastParaBlockNumber,
	}).Debug("Searching backwards from latest block on parachain to find block with nonce")

	currentBlockNumber := lastParaBlockNumber + 1
	basicNonceFound := false
	incentivizedNonceFound := false
	var blocks []ParaBlockWithDigest
	for (!basicNonceFound || !incentivizedNonceFound) && currentBlockNumber != 0 {
		currentBlockNumber--
		log.WithFields(log.Fields{
			"blockNumber": currentBlockNumber,
		}).Debug("Checking header...")

		blockHash, err := li.relaychainConn.API().RPC.Chain.GetBlockHash(currentBlockNumber)
		if err != nil {
			log.WithFields(log.Fields{
				"blockNumber": currentBlockNumber,
			}).WithError(err).Error("Failed to fetch blockhash")
			return nil, err
		}

		storageKey, err := types.CreateStorageKey(li.relaychainConn.Metadata(), "LeafProvider", "LatestDigest")
		if err != nil {
			log.WithError(err).Error("Failed to create storage key")
		}
		latestDigest := types.Digest{}

		ok, err := li.relaychainConn.API().RPC.State.GetStorage(storageKey, &latestDigest, blockHash)
		if err != nil {
			log.WithError(err).Error("Failed to get digest")
			return nil, err
		}
		if !ok {
			continue
		}
		ownParachainHeadBytes, err := types.EncodeToBytes(latestDigest)
		if err != nil {
			return nil, err
		}
		computedDigestHash := keccak.New().Hash(ownParachainHeadBytes)
		computedDigestHashString := hex.EncodeToString(computedDigestHash)
		log.WithFields(log.Fields{"digest": latestDigest, "hash": computedDigestHashString}).Info("Get digest")

		digestItems, err := relaychain.ExtractAuxiliaryDigestItems(latestDigest)
		if err != nil {
			return nil, err
		}

		var digestItemsWithData []DigestItemWithData

		for _, digestItem := range digestItems {
			if digestItem.IsCommitment {
				channelID := digestItem.AsCommitment.ChannelID
				if channelID.IsBasic && !basicNonceFound {
					isRelayed, messageData, err := li.checkBasicMessageNonces(&digestItem, basicNonceToFind)
					if err != nil {
						return nil, err
					}
					if isRelayed {
						basicNonceFound = true
					} else {
						item := DigestItemWithData{digestItem, messageData}
						digestItemsWithData = append(digestItemsWithData, item)
					}
				}
				if channelID.IsIncentivized && !incentivizedNonceFound {
					isRelayed, messageData, err := li.checkIncentivizedMessageNonces(&digestItem, incentivizedNonceToFind)
					if err != nil {
						return nil, err
					}
					if isRelayed {
						incentivizedNonceFound = true
					} else {
						item := DigestItemWithData{digestItem, messageData}
						digestItemsWithData = append(digestItemsWithData, item)
					}
				}
			}
		}

		if len(digestItemsWithData) != 0 {
			block := ParaBlockWithDigest{
				BlockNumber:         currentBlockNumber,
				DigestItemsWithData: digestItemsWithData,
				Digest:              latestDigest,
			}
			blocks = append(blocks, block)
		}
	}

	return blocks, nil
}

func (li *BeefyListener) checkBasicMessageNonces(
	digestItem *relaychain.AuxiliaryDigestItem,
	nonceToFind uint64,
) (bool, types.StorageDataRaw, error) {
	messages, data, err := li.relaychainConn.GetBasicOutboundMessages(*digestItem)
	if err != nil {
		return false, nil, err
	}

	for _, message := range messages {
		if message.Nonce <= nonceToFind {
			return true, data, nil
		}
	}
	return false, data, nil
}

func (li *BeefyListener) checkIncentivizedMessageNonces(
	digestItem *relaychain.AuxiliaryDigestItem,
	nonceToFind uint64,
) (bool, types.StorageDataRaw, error) {

	messages, data, err := li.relaychainConn.GetIncentivizedOutboundMessages(*digestItem)
	if err != nil {
		return false, nil, err
	}

	for _, message := range messages {
		if message.Nonce <= nonceToFind {
			return true, data, nil
		}
	}
	return false, data, nil
}
