#[cfg(test)]
mod treasure_tests {
    use magik_program::state::{Treasure, Vault};
    use solana_sdk::{signature::Keypair, signer::Signer};

    #[test]
    fn test_use_earned_yield() {
        let inputs = vec![
            (
                create_treasure(10, 0, 0),
                create_vault(100, 110),
                (1, 0, 11, 110),
            ),
            (
                create_treasure(10, 5, 0),
                create_vault(100, 110),
                (0, 4, 11, 110),
            ),
        ];

        for ref mut input in inputs {
            let ref mut t = input.0;
            let expected = input.2;
            t.use_earned_yield(&input.1);

            assert_eq!(t.current_credit, expected.0);
            assert_eq!(t.current_borrow, expected.1);
            assert_eq!(t.total_earned_yield, expected.2);
            assert_eq!(t.last_known_vault_yield, expected.3);
        }
    }

    #[test]
    fn test_draw_credit() {
        let inputs = vec![
            (create_treasure(10, 0, 0), 1, (0, 1)),
            (create_treasure(10, 3, 0), 1, (0, 4)),
            (create_treasure(10, 0, 5), 1, (4, 0)),
            (create_treasure(10, 0, 5), 6, (0, 1)),
        ];

        for ref mut input in inputs {
            let ref mut t = input.0;
            let expected = input.2;
            t.draw_credit(input.1);

            assert_eq!(t.current_credit, expected.0);
            assert_eq!(t.current_borrow, expected.1);
        }
    }

    fn create_treasure(deposit: u64, borrow: u64, credit: u64) -> Treasure {
        Treasure {
            current_deposit: deposit,
            current_borrow: borrow,
            current_credit: credit,
            total_earned_yield: 10,
            last_known_vault_yield: 100,
        }
    }

    fn create_vault(deposit: u64, yield_harvested: u64) -> Vault {
        let dummy_pubkey = Keypair::new().pubkey();
        Vault {
            bump: 1,
            payer: dummy_pubkey,
            mint_token: dummy_pubkey,
            vault_token: dummy_pubkey,
            synth_token: dummy_pubkey,
            percent: 25,
            total_deposit: deposit,
            total_yield_harvested: yield_harvested,
        }
    }
}
