This is my code for the task written in Rust. 
Just run 
```bash 
cargo build.
```
The python server run through 
```bash
python buggy_server.py
```
Then copy the length of data and the hash and run the rust client like 
```bash
./target/debug/glitchy-http <length> [<hash>]
```
ie 
```bash
./target/debug/glitchy-http 646863 2dd68fc089b24751559de2d45463341a780dd388f70d4053a5d49cef2cc19e6a
``` 
Optionally you can also omit the expected hash and the hash will be outputted.
```bash
./target/debug/glitchy-http 646863
```

# My Approach 
Take in the expected length and hash as command line arguments. 
Download the full data in chunks of a controllable size set in client.rs. 
Get the SHA-256 hash using the sha-2 crate and then compare with the expected hash.

## External Library Notice for SHA-256 implementation
The task did say to try not to use any external libraries however 
Rust doesn't have a built SHA-256 implementation while Kotlin does
via java.security. MessageDigest so I think it's only fair to use it for Rust. 

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

Also we take in the content length on the command line simply because my logic can't quite decouple the logic 
of extracting the content-length without requesting a lot of bytes. Causes a network error in my program. 
Could be worth looking into to improve the ergonomics of the program but it isn't the biggest deal.

### Scenario 1: Threshold gets smaller but stays constant
In this case I can just adjust my CHUNK_SIZE constant.
It'd be pretty fast to find the threshold through manual testing of a black-box web server. 

### Scenario 2: Threshold changes however threshold varies for seemingly no reason
In this instance, some of my code would need to be overhauled to also perhaps do some binary/linear search 
of a good CHUNK_SIZE to do it on. ie start with a large value for CHUNK_SIZE and reduce it at runtime everytime
a truncation occurs. We can tell if truncation occurs because the Content-Length returned will be not what we
expected. Of course, CHUNK_SIZE will have to stop being a compile time constant and instead be a runtime constant.
I'd change my logic to not expect x amount of bytes per chunk and instead make one of my functions return
how many bytes it did receive up to. Therefore each "chunk" can be of variable size

## Potential Improvements
Other than making my code more general and being able to handle the aforementioned scenario 2. 
If I could use external libraries: I'd make my code multi-threaded using an async-runtime such as tokio 
or even normal thread spawning
though multi-threaded code together with Scenario 2 handling would be pretty tricky. 
It's much easier to make it multi-threaded with constant size chunks.
Update: I just realised that the python server is single-threaded so making the multi-threaded client 
won't make any difference as the server will still process things sequentially.
