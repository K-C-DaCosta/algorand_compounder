# Algorand Auto Compounder
A small program that automatically compounds based on your balance.

# Why the compounder?
I wrote this because I thought collecting manually was just too much of a pain. Personally, I just have this running on a raspberry pi 4 in a background process, so i can just set it up once and forget it.  

# Warning
**this program requires you to input your secret 25 word mnemonic**.
In algorand, you collect rewards by:
* sending zero-transactions to yourself (software requires a private key)
* by recieveing a transaction from someone else(no private key required, but, you usually have to send algo to a third party). 

This program collects rewards by sending zero-transactions. So, it won't cost you algo to use, but you've got to trust that this program does the right thing, which should be relatively easy since the program is quite small. 

# How to run 
make sure the enviroment-variable `ALGORAND_DATA` is properly set before running the program then do:
```
cargo run
```
enter your 25 word mnemonic and enjoy.


** EDIT: **  the formula im using is wrong. I gotta fix that at somepoint. 
