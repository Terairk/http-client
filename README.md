This is my code for the task written in Rust. 

## External Library Notice for SHA-256 implementation
The task did say to try not to use any external libraries however 
Rust doesn't have a built SHA-256 implementation while Kotlin does
via java.security.MessageDigest so I think it's only fair to use it for Rust. 

Furthermore, implementing SHA-256 by hand is error prone (though it would be nice 
as a coding exercise but for now I'll use a crate for maximum security and correctness).

## Pitfalls found 
I've found just how buggy the python server really is. 
The python server doesn't properly respect the Range header treating start-end as exclusive range
while the [current standard](https://www.rfc-editor.org/rfc/rfc7233#section-2.1) states that the byte positions are inclusive. 
Furthermore, it doesn't specify Content-Range either ie Content-Range: bytes 0-499/1234 to tell you how many 
bytes remain but this isn't as bad.

## Assumptions
Also my code assumes that we know the server implementation and know how it works, 
so we use chunks less than the truncated threshold of 64 KiB. 
If we didn't know these, the code would be more complicated however it depends on what gets changed.

###
Scenario 1: Threshold gets smaller but stays constant, in this case I can just adjust my CHUNK_SIZE constant.
It'd be pretty fast to find the threshold through manual testing of a black-box web server. 

###
Scenario 2: Threshold changes however threshold varies for seemingly no reason
In this instance, some of my code would need to be overhauled to also perhaps do some binary search 
of a good CHUNK_SIZE to do it on. ie start with a large value for CHUNK_SIZE and reduce it at runtime everytime
a truncation occurs. We can tell if truncation occurs because the Content-Length returned will be not what we
expected. Ofc CHUNK_SIZE will have to stop being a compile time constant and instead be a runtime constant.
