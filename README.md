[![crates.io](http://meritbadge.herokuapp.com/dominator)](https://crates.io/crates/dominator)
[![docs.rs](https://docs.rs/dominator/badge.svg)](https://docs.rs/dominator)

Zero cost declarative DOM library using FRP signals for Rust!

Status
======

It is generally feature complete, though more convenience methods might be added over time.

It is quite stable: breaking changes are very rare, and are handled with the normal semver system.

I have successfully used Dominator on multiple large applications, and it performed excellently.

Dominator is one of the fastest DOM frameworks in the world ([it is just as fast as Inferno][benchmark]),
and it scales incredibly well even with very large applications. Dominator is so fast that it is
almost never the bottleneck, instead the real bottleneck is the browser itself.

Running the examples
====================

Just do `yarn` and then `yarn start` (it will take a while to compile the dependencies, please be patient)

Community
=========

We have a [Discord server](https://discord.gg/fDFGvnR). Feel free to ask any Dominator questions there.

[benchmark]: https://rawgit.com/krausest/js-framework-benchmark/master/webdriver-ts-results/table.html
