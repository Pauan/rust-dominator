## 0.5.34 - (2023-08-24)
* Adding in new `attrs!`, `props!`, and `styles!` macros which are shorthands for `.attr` / `.prop` / `.style` respectively.

## 0.5.33 - (2023-06-02)
* The `apply_methods!` macro now works with macro paths, like `.path::to::foo!()`

## 0.5.32 - (2023-02-03)
* Adding in `replace_dom` function.

## 0.5.31 - (2022-10-09)
* Adding in `window_size()` function.
* Adding in fragments.

## 0.5.30 - (2022-09-09)
* Adding in `error` event.

## 0.5.29 - (2022-09-05)
* Improves runtime error messages.
* Adds in new `#![prefix = "foo"]` syntax for `class!`. This is used to give a name to the class, for debugging purposes.
* Adding in new `SelectionChange` event.

## 0.5.28 - (2022-07-16)
* Fixing the incorrect behavior of MouseEnter and MouseLeave events.

## 0.5.27 - (2022-07-05)
* Replacing inline_js with js-sys and gloo_events.

## 0.5.26 - (2022-06-13)
* Removing string interning from `attr` values, it was causing major performance issues.

## 0.5.25 - (2022-06-10)
* Adding in `submit` event.
* Adding in `touchstart`, `touchmove`, `touchend`, and `touchcancel` events.
* Fixing compile errors with `nightly` feature.

## 0.5.24 - (2022-04-12)
* Adding in `AnimationStart`, `AnimationIteration`, `AnimationCancel`, `AnimationEnd`, `FocusIn`, `FocusOut`, and `Wheel` events.
* Adding in numerous `Pointer*` events.
* The `apply_methods`, `html`, and `svg` macros now support generics such as `.foo::<T>()`.
* Deprecating the `attribute`, `attribute_namespace`, `attribute_signal` and `attribute_namespace_signal` methods of `DomBuilder`.
* Deprecating the `property` and `property_signal` methods of `DomBuilder`.
* Adding in `AsStr` support for `Cow`.

## 0.5.23 - (2021-12-09)
* Adding in MouseEvent methods to the DragEvents.

## 0.5.22 - (2021-10-25)
* Adding in `movement_x`, `movement_y`, `page_x`, `page_y`, `offset_x`, and `offset_y` to the mouse events.
