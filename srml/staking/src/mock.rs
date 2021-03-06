use std::{cell::RefCell, collections::HashSet};

use sr_primitives::{
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, Convert, IdentityLookup, OnInitialize, OpaqueKeys},
	Perbill,
};
use sr_staking_primitives::SessionIndex;
use srml_support::{
	assert_ok, impl_outer_origin, parameter_types,
	traits::{Currency, Get},
	StorageLinkedMap,
};
use substrate_primitives::H256;

use crate::{
	phragmen::ExtendedBalance, EraIndex, GenesisConfig, Module, Nominators, RewardDestination, StakerStatus,
	StakingBalance, Trait,
};

/// The AccountId alias in this test module.
pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u64;

/// Simple structure that exposes how u64 currency can be represented as... u64.
pub struct CurrencyToVoteHandler;
impl Convert<u64, u64> for CurrencyToVoteHandler {
	fn convert(x: u64) -> u64 {
		x
	}
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
	fn convert(x: u128) -> u64 {
		x as u64
	}
}

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
	static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
}

pub struct TestSessionHandler;
impl session::SessionHandler<AccountId> for TestSessionHandler {
	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		_changed: bool,
		validators: &[(AccountId, Ks)],
		_queued_validators: &[(AccountId, Ks)],
	) {
		SESSION.with(|x| *x.borrow_mut() = (validators.iter().map(|x| x.0.clone()).collect(), HashSet::new()));
	}

	fn on_disabled(validator_index: usize) {
		SESSION.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index];
			d.1.insert(value);
		})
	}
}

pub fn is_disabled(validator: AccountId) -> bool {
	SESSION.with(|d| d.borrow().1.contains(&validator))
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
	fn get() -> u64 {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

impl_outer_origin! {
	pub enum Origin for Test {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}
parameter_types! {
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 0;
	pub const TransactionBaseFee: u64 = 0;
	pub const TransactionByteFee: u64 = 0;
}
impl balances::Trait for Test {
	type Balance = u64;
	type OnFreeBalanceZero = Staking;
	type OnNewAccount = ();
	type TransferPayment = ();
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}
parameter_types! {
	pub const Period: BlockNumber = 1;
	pub const Offset: BlockNumber = 0;
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl session::Trait for Test {
	type Event = ();
	type ValidatorId = AccountId;
	type ValidatorIdOf = crate::StashOf<Test>;
	type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
	type OnSessionEnding = session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = TestSessionHandler;
	type Keys = UintAuthorityId;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type SelectInitialValidators = Staking;
}

impl session::historical::Trait for Test {
	type FullIdentification = crate::Exposure<AccountId, ExtendedBalance>;
	type FullIdentificationOf = crate::ExposureOf<Test>;
}
parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
}

impl kton::Trait for Test {
	type Balance = Balance;
	type Event = ();
	type OnMinted = ();
	type OnRemoval = ();
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 3;
	pub const BondingDuration: EraIndex = 3;
	pub const ErasPerEpoch: EraIndex = 10;
}
pub const COIN: u64 = 1_000_000_000;
parameter_types! {
	// decimal 9
	pub const CAP: Balance = 10_000_000_000 * COIN;
}
impl Trait for Test {
	type Ring = Ring;
	type Kton = Kton;
	type CurrencyToVote = CurrencyToVoteHandler;
	type Event = ();
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

pub struct ExtBuilder {
	existential_deposit: u64,
	current_era: EraIndex,
	reward: u64,
	validator_pool: bool,
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	fair: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 0,
			current_era: 0,
			reward: 10,
			validator_pool: false,
			nominate: true,
			validator_count: 3,
			minimum_validator_count: 0,
			fair: true,
		}
	}
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn _current_era(mut self, current_era: EraIndex) -> Self {
		self.current_era = current_era;
		self
	}
	pub fn validator_pool(mut self, validator_pool: bool) -> Self {
		self.validator_pool = validator_pool;
		self
	}
	pub fn nominate(mut self, nominate: bool) -> Self {
		self.nominate = nominate;
		self
	}
	pub fn validator_count(mut self, count: u32) -> Self {
		self.validator_count = count;
		self
	}
	pub fn minimum_validator_count(mut self, count: u32) -> Self {
		self.minimum_validator_count = count;
		self
	}
	pub fn fair(mut self, is_fair: bool) -> Self {
		self.fair = is_fair;
		self
	}
	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
	}
	pub fn build(self) -> runtime_io::TestExternalities {
		self.set_associated_consts();
		let mut storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let balance_factor = if self.existential_deposit > 0 {
			1_000 * COIN
		} else {
			1 * COIN
		};
		let validators = if self.validator_pool {
			vec![10, 20, 30, 40]
		} else {
			vec![10, 20]
		};

		let _ = session::GenesisConfig::<Test> {
			keys: validators.iter().map(|x| (*x, UintAuthorityId(*x))).collect(),
		}
		.assimilate_storage(&mut storage);

		let _ = balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10 * balance_factor),
				(2, 20 * balance_factor),
				(3, 300 * balance_factor),
				(4, 400 * balance_factor),
				(10, balance_factor),
				(11, balance_factor * 1000),
				(20, balance_factor),
				(21, balance_factor * 2000),
				(30, balance_factor),
				(31, balance_factor * 2000),
				(40, balance_factor),
				(41, balance_factor * 2000),
				(100, 2000 * balance_factor),
				(101, 2000 * balance_factor),
			],
			vesting: vec![],
		}
		.assimilate_storage(&mut storage);

		let _ = kton::GenesisConfig::<Test> {
			balances: vec![],
			vesting: vec![],
		}
		.assimilate_storage(&mut storage);

		let stake_21 = if self.fair { 1000 } else { 2000 };
		let stake_31 = if self.validator_pool { balance_factor * 1000 } else { 1 };
		let status_41 = if self.validator_pool {
			StakerStatus::<AccountId>::Validator
		} else {
			StakerStatus::<AccountId>::Idle
		};
		let nominated = if self.nominate { vec![11, 21] } else { vec![] };
		let _ = GenesisConfig::<Test> {
			current_era: self.current_era,
			current_era_total_reward: 80_000_000 * COIN / ErasPerEpoch::get() as u64,
			stakers: vec![
				//                (2, 1, 1 * COIN, StakerStatus::<AccountId>::Validator),
				(11, 10, 100 * COIN, StakerStatus::<AccountId>::Validator),
				(21, 20, stake_21, StakerStatus::<AccountId>::Validator),
				(31, 30, stake_31, StakerStatus::<AccountId>::Validator),
				(41, 40, balance_factor * 1000, status_41),
				// nominator
				(
					101,
					100,
					balance_factor * 500,
					StakerStatus::<AccountId>::Nominator(nominated),
				),
			],
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			session_reward: Perbill::from_rational_approximation(1_000_000 * self.reward / balance_factor, 1_000_000),
			offline_slash: Perbill::from_percent(5),
			offline_slash_grace: 0,
			invulnerables: vec![],
		}
		.assimilate_storage(&mut storage);

		let mut ext = runtime_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			let validators = Session::validators();
			SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
		});
		ext
	}
}
pub type System = system::Module<Test>;
pub type Ring = balances::Module<Test>;
pub type Kton = kton::Module<Test>;
pub type Session = session::Module<Test>;
pub type Timestamp = timestamp::Module<Test>;
pub type Staking = Module<Test>;

pub fn check_exposure_all() {
	Staking::current_elected()
		.into_iter()
		.for_each(|acc| check_exposure(acc));
}

pub fn check_nominator_all() {
	<Nominators<Test>>::enumerate().for_each(|(acc, _)| check_nominator_exposure(acc));
}

/// Check for each selected validator: expo.total = Sum(expo.other) + expo.own
pub fn check_exposure(stash: u64) {
	assert_is_stash(stash);
	let expo = Staking::stakers(&stash);
	assert_eq!(
		expo.total as u128,
		expo.own as u128 + expo.others.iter().map(|e| e.value as u128).sum::<u128>(),
		"wrong total exposure for {:?}: {:?}",
		stash,
		expo,
	);
}

/// Check that for each nominator: slashable_balance > sum(used_balance)
/// Note: we might not consume all of a nominator's balance, but we MUST NOT over spend it.
pub fn check_nominator_exposure(stash: u64) {
	assert_is_stash(stash);
	let mut sum = 0;
	Staking::current_elected()
		.iter()
		.map(|v| Staking::stakers(v))
		.for_each(|e| e.others.iter().filter(|i| i.who == stash).for_each(|i| sum += i.value));
	let nominator_stake = Staking::slashable_balance_of(&stash);
	// a nominator cannot over-spend.
	assert!(
		nominator_stake >= sum,
		"failed: Nominator({}) stake({}) >= sum divided({})",
		stash,
		nominator_stake,
		sum,
	);
}

pub fn assert_total_expo(stash: u64, val: u128) {
	let expo = Staking::stakers(&stash);
	assert_eq!(expo.total, val);
}

pub fn assert_is_stash(acc: u64) {
	assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

pub fn bond_validator(acc: u64, val: u64) {
	// a = controller
	// a + 1 = stash
	let _ = Ring::make_free_balance_be(&(acc + 1), val);
	assert_ok!(Staking::bond(
		Origin::signed(acc + 1),
		acc,
		StakingBalance::Ring(val),
		RewardDestination::Controller,
		0
	));
	assert_ok!(Staking::validate(
		Origin::signed(acc),
		"test".as_bytes().to_owned(),
		0,
		0
	));
}

pub fn bond_nominator(acc: u64, val: u64, target: Vec<u64>) {
	// a = controller
	// a + 1 = stash
	let _ = Ring::make_free_balance_be(&(acc + 1), val);
	assert_ok!(Staking::bond(
		Origin::signed(acc + 1),
		acc,
		StakingBalance::Ring(val),
		RewardDestination::Controller,
		0
	));
	assert_ok!(Staking::nominate(Origin::signed(acc), target));
}

pub fn start_session(session_index: SessionIndex) {
	for i in 0..(session_index - Session::current_index()) {
		System::set_block_number((i + 1).into());
		Session::on_initialize(System::block_number());
	}
	assert_eq!(Session::current_index(), session_index);
}

pub fn start_era(era_index: EraIndex) {
	start_session((era_index * 3).into());
	assert_eq!(Staking::current_era(), era_index);
}
