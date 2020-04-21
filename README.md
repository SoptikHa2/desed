# Desed
Demystify and debug your sed scripts, from comfort of your terminal.

![desed usage example](img/desed.gif)

Desed is a command line tool with beautiful TUI that provides users with comfortable interface and practical debugger, used to step through complex sed scripts.

Some of the notable features include:

- Preivew variable values, both of them!
- See how will a substitute command affect pattern space before it runs
- Step through sed script - both forward and backwards!
- Place breakpoints and examine program state
- It's name is a palindrome

## Install

From source:
```
git clone https://github.com/soptikha2/desed
cd desed
cargo install --path .
```

From cargo:
```
cargo install desed
```

### Dependencies:

Development: `rust`, `cargo`

Runtime: `sed` (GNU version)

## Controls

- Mouse scroll to scroll through source code, click on line to toggle breakpoint
- `j`, `k`, `g`, `G`, just as in Vim. Prefixing with numbers work too.
- `b` to toggle breakpoint (prefix with number to toggle breakpoint on target line)
- `s` to step forward, `a` to step backwards
- `r` to run to next breakpoint or end of script, `R` to do the same but backwards
- `l` to instantly reload code and continue debugging in the exactly same place as before
- `q` to [quit](https://github.com/hakluke/how-to-exit-vim)

# FAQ

## How does it work?
GNU sed actually provides pretty useful debugging interface, try it yourself with `--debug` flag. However the interface is not interactive and I wanted something closer to traditional debugger.

## Does it really work?
Depends. Sed actually doesn't tell me which line number is it currently executing, so I have to emulate parts of sed to guess that. Which might not be bulletproof. But it certainly worked good enough to debug tetris without issues.

## Why sed??

Sed is the perfect programming language, [especially for graph problems](https://tildes.net/~comp/b2k/programming_challenge_find_path_from_city_a_to_city_b_with_least_traffic_controls_inbetween#comment-2run). It's plain and simple and doesn't clutter your screen with useless identifiers like `if`, `for`, `while`, or `int`. Furthermore since it doesn't have things like numbers, it's very simple to use.

## But why?

I wanted to program in sed but it lacked good tooling up to this point, so I had to do something about it.

## Why?

Because it's the standard stream editor for filtering and transforming text. And someone wrote [tetris](https://github.com/uuner/sedtris) in it!

## What is the roadmap for future updates?

I would like to introduce syntax highlighting and add this tools to standard repositories of all major distributions.
