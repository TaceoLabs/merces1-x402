
-- %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
-- %            Transaction type enum                 %
-- %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

CREATE TYPE transaction_type AS ENUM ('deposit', 'withdraw', 'transfer', 'transfer_from');

-- %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
-- %                  Transactions                    %
-- %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

CREATE TABLE transactions (
    id             BIGSERIAL        PRIMARY KEY,
    sender         BYTEA            NOT NULL,
    receiver       BYTEA            NOT NULL,
    type           transaction_type NOT NULL,
    tx_hash        BYTEA,
    amount         TEXT,
    amount_commitment TEXT,
    amount_share   TEXT,
    "timestamp"    TIMESTAMPTZ      NOT NULL DEFAULT now(),

    -- deposit/withdraw: plain amount only
    CONSTRAINT chk_deposit_withdraw CHECK (
        type NOT IN ('deposit', 'withdraw')
        OR (amount IS NOT NULL AND amount_commitment IS NULL AND amount_share IS NULL)
    ),

    -- transfer/transfer_from: commitment + share, no plain amount
    CONSTRAINT chk_transfer CHECK (
        type NOT IN ('transfer', 'transfer_from')
        OR (amount IS NULL AND amount_commitment IS NOT NULL AND amount_share IS NOT NULL)
    )
);
