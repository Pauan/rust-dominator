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
