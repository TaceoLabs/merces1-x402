[private]
default:
    @just --justfile {{ justfile() }} --list --list-heading $'Project commands:\n'

[working-directory('contracts')]
show-contract-errors:
    forge inspect src/Merces.sol:Merces errors
    forge inspect src/verifiers/VerifierClient.sol:Verifier errors
    forge inspect src/verifiers/VerifierServer.sol:Verifier errors
    forge inspect src/Token.sol:USDCToken errors
    forge inspect src/Token.sol:USDT0Token errors
