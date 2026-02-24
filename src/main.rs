mod bintree;

use colored::*;
use crypto_rs::secp256k1::{Secp256k1Point, Secp256k1Scalar};
use nested_musig2::{keyagg::key_agg, keygen::keygen, params::Params, round1::{Round1Out, Round1State, sign_agg, sign_agg_ext, sign_round1}, round2::{sign_agg_prime, sign_prime, ver}};
use std::{collections::HashMap, io};

use crate::bintree::BinTree;

struct NodeState {
    secret_key: Option<Secp256k1Scalar>,
    state: Option<Round1State>,
    out: Option<Round1Out>,
    out_internal: Option<Round1Out>,
    out_prime: Option<Secp256k1Scalar>,
    state_prime: Option<Secp256k1Point>,
}

fn round1(node: &BinTree<Secp256k1Point>, state_map: &mut HashMap<Secp256k1Point, NodeState>) {
    match node {
        BinTree::Leaf(pk) => {
            let (out, _state) = sign_round1(2).unwrap();
            if let Some(state) = state_map.get_mut(&pk) {
                state.out = Some(out);
                state.state = Some(_state);
            }
        },
        BinTree::Node { left, right, value } => {
            round1(left, state_map);
            let mut out_internal = state_map.get(left.value()).unwrap().out.clone().unwrap();
            if let Some(node) = right.as_ref().as_ref() {
                round1(node, state_map);
                let right_out = state_map.get(node.value()).unwrap().out.clone().unwrap();
                out_internal = sign_agg(&[out_internal, right_out]).unwrap();
            }
            let out = sign_agg_ext(&Params::default(), &out_internal, value).unwrap();
            let state = NodeState {
                secret_key: None,
                state: None,
                out: Some(out),
                out_internal: Some(out_internal),
                out_prime: None,
                state_prime: None,
            };
            state_map.insert(value.clone(), state);
        }
    }
}

fn round2(node: &BinTree<Secp256k1Point>, state_map: &mut HashMap<Secp256k1Point, NodeState>, msg: &[u8], outs_by_depth: &[Round1Out], merkle_path: Vec<Vec<Secp256k1Point>>) {
    let params = Params::default();
    match node {
        BinTree::Leaf(pk) => {
            let state = state_map.get_mut(pk).unwrap(); 
            let state1 = state.state.clone().unwrap();
            let sk = state.secret_key.clone().unwrap();
            let (state_prime, out_prime) = sign_prime(&params, state1, outs_by_depth, &sk, msg, &merkle_path).unwrap();
            state.out_prime = Some(out_prime);
            state.state_prime = Some(state_prime);
        },
        BinTree::Node { left, right, value } => {
            let state = state_map.get(value).unwrap(); 
            let out_d = state.out_internal.clone().unwrap();

            let mut ext_outs = outs_by_depth.to_vec();
            ext_outs.push(out_d);
            if let Some(r_node) = right.as_ref().as_ref() {

                // insert corresponding pubkeys of siblings at level `lambda`
                let mut l_path = merkle_path.clone();
                let mut r_path = merkle_path.clone();
                r_path.push(vec![left.value().clone()]);
                l_path.push(vec![r_node.value().clone()]);
                round2(left, state_map, msg, &ext_outs, l_path);
                round2(r_node, state_map, msg, &ext_outs, r_path);

                let l_state = state_map.get(left.value()).unwrap().state_prime.clone().unwrap();
                let l_out = state_map.get(left.value()).unwrap().out_prime.clone().unwrap();

                let r_state = state_map.get(r_node.value()).unwrap().state_prime.clone().unwrap();
                let r_out = state_map.get(r_node.value()).unwrap().out_prime.clone().unwrap();

                let parts = &[(l_state, l_out), (r_state, r_out)];
                let (state_prime, out_prime) = sign_agg_prime(parts).unwrap();

                let state = state_map.get_mut(value).unwrap(); 
                state.out_prime = Some(out_prime);
                state.state_prime = Some(state_prime);
            } else {
                // FIXME
                panic!("Should not reach here");
            }
        },
    }
}

fn main() {
    println!(
        "{}",
        "Demonstration of converting any n of n musig to binary tree merkelized nested musig"
            .green()
    );
    println!("Enter {}", "n".yellow());

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let n: u32 = input.trim().parse().unwrap();

    let keys: Vec<_> = (0..n).map(|_| keygen()).collect();
    let mut state_map: HashMap<Secp256k1Point, NodeState> = HashMap::new();
    let mut pubkeys = Vec::new();

    for kp in keys {
        pubkeys.push(kp.pk.clone());
        state_map.insert(
            kp.pk,
            NodeState {
                secret_key: Some(kp.sk),
                state: None,
                out: None,
                out_internal: None,
                out_prime: None,
                state_prime: None,
            },
        );
    }

    println!("Created n keypairs");

    let btree = BinTree::from_vec(pubkeys, |k1, k2| {
        key_agg(&Params::default(), &[k1, k2]).unwrap()
    });

    round1(&btree, &mut state_map);
    let msg = b"test tx message";
    round2(&btree, &mut state_map, msg , &[], vec![]);
    let root_pk = btree.value();
    let state = state_map.get(root_pk).unwrap();
    let sig = (state.state_prime.clone().unwrap(), state.out_prime.clone().unwrap());

    if ver(&Params::default(), root_pk, msg, &sig) {
        println!("{}", "SUCCESS".green());
    } else {
        println!("{}", "FAIL".red());
    }
}
