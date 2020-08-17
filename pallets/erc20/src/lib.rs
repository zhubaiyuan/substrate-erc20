use sp_std::prelude::*;
use codec::{Codec, Encode, Decode};
use frame_support::{Parameter, decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult, ensure};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{CheckedAdd, CheckedSub, Member, AtLeast32BitUnsigned};

// the module trait contains type definitions
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: CheckedAdd + CheckedSub + Parameter + Member + Codec + Default + Copy + AtLeast32BitUnsigned;
}

// struct storeS the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20Token<U> {
    name: Vec<u8>,
    ticker: Vec<u8>,
    total_supply: U,
}

// storage for this module
decl_storage! {
    trait Store for Module<T: Trait> as Erc20 {
        // details of the token corresponding to a token id
        Tokens get(fn token_details): Erc20Token<T::TokenBalance>;
        // balances mapping for an account and token
        BalanceOf get(fn balance_of): map hasher(blake2_128_concat) T::AccountId => T::TokenBalance;
        // allowance for an account and token
        Allowance get(fn allowance): map hasher(blake2_128_concat) (T::AccountId, T::AccountId) => T::TokenBalance;
    }
}

// events
decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId, <T as Trait>::TokenBalance {
        // event for transfer of tokens
        // from, to, value
        Transfer(AccountId, AccountId, TokenBalance),
        // event when an approval is made
        // owner, spender, value
        Approval(AccountId, AccountId, TokenBalance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        StorageOverflow,
    }
}

// public interface for this runtime module
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        // initialize the default event for this module
        fn deposit_event() = default;

        // initializes a new token
        // generates an integer token_id so that all tokens are unique
        // takes a name, ticker, total supply for the token
        // makes the initiating account the owner of the token
        // the balance of the owner is set to total supply
        #[weight = 0]
        fn init(origin, name: Vec<u8>, ticker: Vec<u8>, total_supply: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // checking max size for name and ticker
            // byte arrays (vecs) with no max size should be avoided
            ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
            ensure!(ticker.len() <= 32, "token ticker cannot exceed 32 bytes");

            let token = Erc20Token {
                name,
                ticker,
                total_supply,
            };

            <Tokens<T>>::set(token);
            <BalanceOf<T>>::insert(sender, total_supply);
  
            Ok(())
        }

        // transfer tokens from one account to another
        // origin is assumed as sender
        #[weight = 0]
        fn transfer(_origin, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            Self::_transfer(sender, to, value)
        }

        // the ERC20 standard transfer_from function
        // implemented in the open-zeppelin way - increase/decrease allownace
        // if approved, transfer from an account to another account without owner's signature
        #[weight = 0]
        pub fn transfer_from(_origin, from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
          let allowance = Self::allowance((from.clone(), to.clone()));
          ensure!(allowance >= value, "Not enough allowance.");
            
          // using checked_sub (safe math) to avoid overflow
          let updated_allowance = allowance.checked_sub(&value).ok_or(Error::<T>::StorageOverflow)?;
          <Allowance<T>>::insert((from.clone(), to.clone()), updated_allowance);

          Self::deposit_event(RawEvent::Approval(from.clone(), to.clone(), value));
          Self::_transfer(from, to, value)
        }

        // approve token transfer from one account to another
        // once this is done, transfer_from can be called with corresponding values
        #[weight = 0]
        fn approve(_origin, spender: T::AccountId, value: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(_origin)?;

            let allowance = Self::allowance((sender.clone(), spender.clone()));
            let updated_allowance = allowance + value;
            <Allowance<T>>::insert((sender.clone(), spender.clone()), updated_allowance);

            Self::deposit_event(RawEvent::Approval(sender.clone(), spender.clone(), value));

            Ok(())
        }
    }
}

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // the ERC20 standard transfer function
    // internal
    fn _transfer(
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> DispatchResult {
        let sender_balance = Self::balance_of(from.clone());
        ensure!(sender_balance >= value, "Not enough balance.");

        let updated_from_balance = sender_balance.checked_sub(&value).ok_or(Error::<T>::StorageOverflow)?;
        let receiver_balance = Self::balance_of(to.clone());
        let updated_to_balance = receiver_balance.checked_add(&value).ok_or(Error::<T>::StorageOverflow)?;
        
        // reduce sender's balance
        <BalanceOf<T>>::insert(from.clone(), updated_from_balance);
        // increase receiver's balance
        <BalanceOf<T>>::insert(to.clone(), updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }
}
