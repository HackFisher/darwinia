// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use authority_discovery_primitives::{AuthorityId as EncodedAuthorityId, Signature as EncodedSignature};
use babe_primitives::{AuthorityId as BabeId, AuthoritySignature as BabeSignature};
pub use balances::Call as BalancesCall;
use sr_api::impl_runtime_apis;
use codec::{Decode, Encode};
pub use contracts::Gas;

//use grandpa::fg_primitives;
//use grandpa::{AuthorityId as GrandpaId, AuthorityWeight as GrandpaWeight};
//use im_online::sr25519::AuthorityId as ImOnlineId;

use node_primitives::{AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature};
use rstd::prelude::*;
use sr_primitives::traits::{self, BlakeTwo256, Block as BlockT, NumberFor, SaturatedConversion, StaticLookup};
use sr_primitives::transaction_validity::TransactionValidity;
use sr_primitives::weights::Weight;
#[cfg(any(feature = "std", test))]
pub use sr_primitives::BuildStorage;

use sr_primitives::{create_runtime_str, generic, impl_opaque_keys, ApplyResult, Perbill};
use substrate_primitives::u32_trait::{_1, _4};

use support::traits::OnUnbalanced;
pub use support::StorageValue;
use support::{
	construct_runtime, parameter_types,
	traits::{Currency, Randomness, SplitTwoWays},
};

pub use timestamp::Call as TimestampCall;
use version::RuntimeVersion;
#[cfg(any(feature = "std", test))]

use version::NativeVersion;

use substrate_primitives::OpaqueMetadata;
use grandpa::AuthorityList as GrandpaAuthorityList;
use grandpa::fg_primitives;
use im_online::sr25519::{AuthorityId as ImOnlineId};
use transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use system::offchain::TransactionSubmitter;

use staking::EraIndex;
pub use staking::StakerStatus;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::{CurrencyToVoteHandler, Author, LinearWeightToFee, TargetedFeeAdjustment};

/// Constant values used within the runtime.
pub mod constants;
use constants::time::*;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node"),
	impl_name: create_runtime_str!("darwinia-node"),
	authoring_version: 2,
	spec_version: 79,
	impl_version: 79,
	apis: RUNTIME_API_VERSIONS,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

//pub struct Author;
//
//impl OnUnbalanced<NegativeImbalance> for Author {
//	fn on_unbalanced(amount: NegativeImbalance) {
//		Balances::resolve_creating(&Authorship::author(), amount);
//	}
//}

pub struct MockTreasury;
impl OnUnbalanced<NegativeImbalance> for MockTreasury {
	fn on_unbalanced(amount: NegativeImbalance) {
		Balances::resolve_creating(&Sudo::key(), amount);
	}
}

pub type DealWithFees = SplitTwoWays<
	Balance,
	NegativeImbalance,
	_4,
	MockTreasury, // 4 parts (80%) goes to the treasury.
	_1,
	Author, // 1 part (20%) goes to the block author.
>;

pub const SECS_PER_BLOCK: BlockNumber = 6;
pub const MINUTES: BlockNumber = 60 / SECS_PER_BLOCK;
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1_000_000_000;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
}

impl system::Trait for Runtime {
	type Origin = Origin;
	type Call = Call;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = Version;
}

impl utility::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

impl indices::Trait for Runtime {
	type AccountIndex = AccountIndex;
	type IsDeadAccount = Balances;
	type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
	type Event = Event;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * COIN;
	pub const TransferFee: Balance = 1 * MILLI;
	pub const CreationFee: Balance = 1 * MILLI;
}

impl balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = (Staking, Session);
	type OnNewAccount = Indices;
	type TransferPayment = ();
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

parameter_types! {
	pub const TransactionBaseFee: Balance = 1 * MILLI;
	pub const TransactionByteFee: Balance = 10 * MICRO;
	// setting this to zero will disable the weight fee.
	pub const WeightFeeCoefficient: Balance = 1_000;
	// for a sane configuration, this should always be less than `AvailableBlockRatio`.
	pub const TargetBlockFullness: Perbill = Perbill::from_percent(25);
}

impl transaction_payment::Trait for Runtime {
	type Currency = Balances;
	type OnTransactionPayment = DealWithFees;
	type TransactionBaseFee = TransactionBaseFee;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = LinearWeightToFee<WeightFeeCoefficient>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<TargetBlockFullness>;
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = babe::ExternalTrigger;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl timestamp::Trait for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
	}
}

impl authorship::Trait for Runtime {
	type FindAuthor = session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = Staking;
}

// NOTE: `SessionHandler` and `SessionKeys` are co-dependent: One key will be used for each handler.
// The number and order of items in `SessionHandler` *MUST* be the same number and order of keys in
// `SessionKeys`.
// TODO: Introduce some structure to tie these together to make it a bit less of a footgun. This
// should be easy, since OneSessionHandler trait provides the `Key` as an associated type. #2858

parameter_types! {
	pub const Period: BlockNumber = 1 * MINUTES;
	pub const Offset: BlockNumber = 0;
}

type SessionHandlers = (Grandpa, Babe, ImOnline, AuthorityDiscovery);

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as system::Trait>::AccountId;
	type ValidatorIdOf = staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type OnSessionEnding = Staking;
	type SessionHandler = SessionHandlers;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type SelectInitialValidators = Staking;
}

impl session::historical::Trait for Runtime {
	type FullIdentification = staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = staking::ExposureOf<Runtime>;
}

parameter_types! {
	pub const SessionsPerEra: sr_staking_primitives::SessionIndex = 5;
	// about 14 days
	pub const BondingDuration: staking::EraIndex = 4032;
	// 365 days * 24 hours * 60 minutes / 5 minutes
	pub const ErasPerEpoch: EraIndex = 105120;
}

impl sudo::Trait for Runtime {
	type Event = Event;
	type Proposal = Call;
}

impl grandpa::Trait for Runtime {
	type Event = Event;
}

impl offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = ();
}

type SubmitTransaction = TransactionSubmitter<ImOnlineId, Runtime, UncheckedExtrinsic>;

parameter_types! {
	pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
}

impl im_online::Trait for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type Call = Call;
	type SubmitTransaction = SubmitTransaction;
	type ReportUnresponsiveness = Offences;
	type SessionDuration = SessionDuration;
}

impl authority_discovery::Trait for Runtime {
	type AuthorityId = BabeId;
}

parameter_types! {
	pub const WindowSize: BlockNumber = 101;
	pub const ReportLatency: BlockNumber = 1000;
}

impl finality_tracker::Trait for Runtime {
	type OnFinalizationStalled = Grandpa;
	type WindowSize = WindowSize;
	type ReportLatency = ReportLatency;
}

parameter_types! {
	pub const ContractTransferFee: Balance = 1 * MILLI;
	pub const ContractCreationFee: Balance = 1 * MILLI;
	pub const ContractTransactionBaseFee: Balance = 1 * MILLI;
	pub const ContractTransactionByteFee: Balance = 10 * MICRO;
	pub const ContractFee: Balance = 1 * MILLI;
	pub const TombstoneDeposit: Balance = 1 * COIN;
	pub const RentByteFee: Balance = 1 * COIN;
	pub const RentDepositOffset: Balance = 1000 * COIN;
	pub const SurchargeReward: Balance = 150 * COIN;
}

impl contracts::Trait for Runtime {
	type Currency = Balances;
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Call = Call;
	type Event = Event;
	type DetermineContractAddress = contracts::SimpleAddressDeterminator<Runtime>;
	type ComputeDispatchFee = contracts::DefaultDispatchFeeComputor<Runtime>;
	type TrieIdGenerator = contracts::TrieIdFromParentCounter<Runtime>;
	type GasPayment = ();
	type RentPayment = ();
	type SignedClaimHandicap = contracts::DefaultSignedClaimHandicap;
	type TombstoneDeposit = TombstoneDeposit;
	type StorageSizeOffset = contracts::DefaultStorageSizeOffset;
	type RentByteFee = RentByteFee;
	type RentDepositOffset = RentDepositOffset;
	type SurchargeReward = SurchargeReward;
	type TransferFee = ContractTransferFee;
	type CreationFee = ContractCreationFee;
	type TransactionBaseFee = ContractTransactionBaseFee;
	type TransactionByteFee = ContractTransactionByteFee;
	type ContractFee = ContractFee;
	type CallBaseFee = contracts::DefaultCallBaseFee;
	type InstantiateBaseFee = contracts::DefaultInstantiateBaseFee;
	type MaxDepth = contracts::DefaultMaxDepth;
	type MaxValueSize = contracts::DefaultMaxValueSize;
	type BlockGasLimit = contracts::DefaultBlockGasLimit;
}

impl system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;

	fn create_transaction<F: system::offchain::Signer<AccountId, Self::Signature>>(
		call: Call,
		public: Self::Public,
		account: AccountId,
		index: Index,
	) -> Option<(Call, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
		let period = 1 << 8;
		let current_block = System::block_number().saturated_into::<u64>();
		let tip = 0;
		let extra: SignedExtra = (
			system::CheckVersion::<Runtime>::new(),
			system::CheckGenesis::<Runtime>::new(),
			system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			system::CheckNonce::<Runtime>::from(index),
			system::CheckWeight::<Runtime>::new(),
			transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			Default::default(),
		);
		let raw_payload = SignedPayload::new(call, extra).ok()?;
		let signature = F::sign(public, &raw_payload)?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl kton::Trait for Runtime {
	type Balance = Balance;
	type Event = Event;
	type OnMinted = ();
	type OnRemoval = ();
}

parameter_types! {
	// decimal 9
	pub const CAP: Balance = 10_000_000_000 * COIN;
}

impl staking::Trait for Runtime {
	type Ring = Balances;
	type Kton = Kton;
	type CurrencyToVote = CurrencyToVoteHandler;
	type Event = Event;
	type RingSlash = ();
	type RingReward = ();
	type KtonSlash = ();
	type KtonReward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type Cap = CAP;
	type ErasPerEpoch = ErasPerEpoch;
	type SessionLength = Period;
	type SessionInterface = Self;
}

impl eos_bridge::Trait for Runtime {
	type Event = Event;
}

impl ethereum_bridge::Trait for Runtime {
	type Event = Event;
	type Ring = Balances;
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = node_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		AuthorityDiscovery: authority_discovery::{Module, Call, Config<T>},
		Authorship: authorship::{Module, Call, Storage},
		Babe: babe::{Module, Call, Storage, Config, Inherent(Timestamp)},
		Balances: balances::{default, Error},
		Contracts: contracts,
		FinalityTracker: finality_tracker::{Module, Call, Inherent},
		Grandpa: grandpa::{Module, Call, Storage, Config, Event},
		ImOnline: im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
		Indices: indices,
		Offences: offences::{Module, Call, Storage, Event},
		RandomnessCollectiveFlip: randomness_collective_flip::{Module, Call, Storage},
		Session: session::{Module, Call, Storage, Event, Config<T>},
		Sudo: sudo,
		System: system::{Module, Call, Storage, Config, Event},
		Timestamp: timestamp::{Module, Call, Storage, Inherent},
		TransactionPayment: transaction_payment::{Module, Storage},
		Utility: utility::{Module, Call, Event},

		Kton: kton,
		Staking: staking::{default, OfflineWorker},
		EOSBridge: eos_bridge::{Storage, Module, Event<T>, Call},
		EthereumBridge: ethereum_bridge::{Storage, Module, Event<T>, Call},
	}
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	system::CheckVersion<Runtime>,
	system::CheckGenesis<Runtime>,
	system::CheckEra<Runtime>,
	system::CheckNonce<Runtime>,
	system::CheckWeight<Runtime>,
	transaction_payment::ChargeTransactionPayment<Runtime>,
	contracts::CheckBlockGasLimit<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, system::ChainContext<Runtime>, Runtime, AllModules>;

impl_runtime_apis! {
	impl sr_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sr_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl block_builder_api::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: inherents::InherentData,
		) -> inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl tx_pool_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			Executive::validate_transaction(tx)
		}
	}

	impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(number: NumberFor<Block>) {
			Executive::offchain_worker(number)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}
	}

	impl babe_primitives::BabeApi<Block> for Runtime {
		fn configuration() -> babe_primitives::BabeConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			babe_primitives::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				secondary_slots: true,
			}
		}
	}

	impl authority_discovery_primitives::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<EncodedAuthorityId> {
			AuthorityDiscovery::authorities().into_iter()
				.map(|id| id.encode())
				.map(EncodedAuthorityId)
				.collect()
		}

		fn sign(payload: &Vec<u8>) -> Option<(EncodedSignature, EncodedAuthorityId)> {
			  AuthorityDiscovery::sign(payload).map(|(sig, id)| {
			(EncodedSignature(sig.encode()), EncodedAuthorityId(id.encode()))
		})
		}

		fn verify(payload: &Vec<u8>, signature: &EncodedSignature, authority_id: &EncodedAuthorityId) -> bool {
			let signature = match BabeSignature::decode(&mut &signature.0[..]) {
				Ok(s) => s,
				_ => return false,
			};

			let authority_id = match BabeId::decode(&mut &authority_id.0[..]) {
				Ok(id) => id,
				_ => return false,
			};

			AuthorityDiscovery::verify(payload, signature, authority_id)
		}
	}

	impl system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance> for Runtime {
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: u64,
			input_data: Vec<u8>,
		) -> contracts_rpc_runtime_api::ContractExecResult {
			use contracts_rpc_runtime_api::ContractExecResult;

			let exec_result = Contracts::bare_call(
				origin,
				dest.into(),
				value,
				gas_limit,
				input_data,
			);
			match exec_result {
				Ok(v) => ContractExecResult::Success {
					status: v.status,
					data: v.data,
				},
				Err(_) => ContractExecResult::Error,
			}
		}

		fn get_storage(
			address: AccountId,
			key: [u8; 32],
		) -> contracts_rpc_runtime_api::GetStorageResult {
			Contracts::get_storage(address, key).map_err(|rpc_err| {
				use contracts::GetStorageError;
				use contracts_rpc_runtime_api::{GetStorageError as RpcGetStorageError};
				/// Map the contract error into the RPC layer error.
				match rpc_err {
					GetStorageError::ContractDoesntExist => RpcGetStorageError::ContractDoesntExist,
					GetStorageError::IsTombstone => RpcGetStorageError::IsTombstone,
				}
			})
		}
	}

	impl transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
		UncheckedExtrinsic,
	> for Runtime {
		fn query_info(uxt: UncheckedExtrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	}

	impl substrate_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}
}
