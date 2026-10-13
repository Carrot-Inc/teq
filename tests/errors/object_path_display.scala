// A type through a val of an object is shown with the object's path, as dotty prints `TermRef(API, b)`:
// `API.b.Item`, where master showed `b.Item`.
object API:
  class B { class Item }
  val b = new B
val x: String = new API.b.Item
// expect: 6:17: error: type mismatch: found API.b.Item, required String
