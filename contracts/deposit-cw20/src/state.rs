use std::marker::PhantomData;

use cw20::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Uint128, Addr, Coin, BlockInfo, CustomMsg};
use cw_storage_plus::{Map, Item, SnapshotItem, IndexedSnapshotMap, SnapshotMap, Strategy, UniqueIndex, Index, IndexList, MultiIndex, IndexedMap};

pub struct Deposit<'a, C>
where
    C: CustomMsg
{
    //keys address and denom
    pub total_deposits: Item<'a, u64>,
    pub deposits: Map<'a, (&'a str, &'a str), Deposits>,

    pub total_cw20_deposits: SnapshotItem<'a, u64>,
    pub cw20_deposits: IndexedMap<'a, (&'a str, &'a str), Cw20Deposits, Cw20DepositIndexes<'a>>,
    //key is contract address, token_id
    pub cw721_deposits: IndexedSnapshotMap<'a, (&'a str, &'a str), Cw721Deposits, Cw721DepositIndexes<'a>>,
    pub(crate) _custom_response: PhantomData<C>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw20Deposits {
    pub count: u64,
    pub owner: String,
    pub contract:String,
    pub amount:Uint128,
    pub stake_time:Expiration
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Deposits {
    pub count: i32,
    pub owner: Addr,
    pub coins: Coin
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw721Deposits {
    pub owner: String,
    pub contract:String,
    pub token_id:String
}

pub struct Cw20DepositIndexes<'a> {
    pub count: MultiIndex<'a, u64, Cw20Deposits, &'a str>,
    pub owner: MultiIndex<'a, String, Cw20Deposits, &'a str>,
}

impl<'a> IndexList<Cw20Deposits> for Cw20DepositIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Cw20Deposits>> + '_> {
        let v: Vec<&dyn Index<Cw20Deposits>> = vec![&self.count, &self.owner];
        Box::new(v.into_iter())
    }
}

pub struct Cw721DepositIndexes<'a> {
    pub owner: MultiIndex<'a, String, Cw721Deposits, &'a str>,
}

impl<'a> IndexList<Cw721Deposits> for Cw721DepositIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Cw721Deposits>> + '_> {
        let v: Vec<&dyn Index<Cw721Deposits>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

impl<C> Default for Deposit<'static, C>
where
    C: CustomMsg
{
    fn default() -> Self {
        Self::new(
            "total_deposits",
            "deposits",
        )
    }
}

impl<'a, C> Deposit<'a, C>
where
    C: CustomMsg
{
    fn new(
        total_deposits_key: &'a str,
        deposits_key: &'a str,
    ) -> Self {
        Self {
            total_deposits: Item::new(total_deposits_key),
            deposits: Map::new(deposits_key),
            total_cw20_deposits: SnapshotItem::new(
                "total_cw20_deposits",
                "total_cw20_deposits_check",
                "total_cw20_deposits_change",
                Strategy::EveryBlock,
            ),
            cw20_deposits: IndexedMap::new(
                "cw20_deposits",
                Cw20DepositIndexes {
                    count: MultiIndex::new(|_pk, d| d.count.clone(), "cw20deposits", "cw20deposits__count"),
                    owner: MultiIndex::new(|_pk, d| d.owner.clone(), "cw20deposits", "cw20deposits__owner")
                },
            ),
            cw721_deposits: IndexedSnapshotMap::new(
                "cw721_deposits",
                "cw721_deposits_check",
                "cw721_deposits_change",
                Strategy::EveryBlock,
                Cw721DepositIndexes { 
                    owner: MultiIndex::new(|_pk, d| d.owner.clone(), "cw721deposits", "cw721deposits__owner")
                }
            ),
            _custom_response: PhantomData,
        }
    }
}


//key is address, denom
//pub const DEPOSITS: Map<(&str, &str), Deposits> = Map::new("deposits");

//key is address, cw20 contract address
//pub const CW20_DEPOSITS: Map<(&str, &str), Cw20Deposits> = Map::new("cw20deposits");

//contract, owner, token_id
//pub const CW721_DEPOSITS: Map<(&str, &str, &str), Cw721Deposits> = Map::new("cw721deposits");
