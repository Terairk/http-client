This is my code for the task written in Rust. 

The task did say to try not to use any external libraries however 
Rust doesn't have a built SHA-256 implementation while Kotlin does
via java.security.MessageDigest so I think it's only fair to use it for Rust. 

Furthermore, implementing SHA-256 by hand is error prone (though it would be nice 
as a coding exercise but for now I'll use a crate for maximum security and correctness).

