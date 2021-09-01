[![crates.io](https://img.shields.io/crates/v/dominator.svg)](https://crates.io/crates/dominator)
[![docs.rs](https://docs.rs/dominator/badge.svg)](https://docs.rs/dominator)

Zero-cost ultra-high-performance declarative DOM library using FRP signals for Rust!

Overview
========

Dominator is one of the fastest DOM frameworks in the world ([it is just as fast as Inferno][benchmark]).

It does not use VDOM, instead it uses raw DOM nodes for maximum performance. It is close to the metal and
has almost no overhead: everything is inlined to raw DOM operations.

It scales incredibly well even with very large applications, because updates are always `O(1)` time, no
matter how big or deeply nested your application is.

It has a convenient high level declarative API which works similar to React components, but is
designed for Rust and FRP signals.

It is generally feature complete, though more convenience methods might be added over time.

It is quite stable: breaking changes are very rare, and are handled with the normal semver system.

I have successfully used Dominator on multiple large applications, and it performed excellently.

Running the examples
====================

Just do `yarn` and then `yarn start` (it will take a while to compile the dependencies, please be patient)

Community
=========

We have a [Discord server](https://discord.gg/fDFGvnR). Feel free to ask any Dominator questions there.

[benchmark]: https://rawgit.com/krausest/js-framework-benchmark/master/webdriver-ts-results/table.html
