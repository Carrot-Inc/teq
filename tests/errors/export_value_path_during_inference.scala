// expect: export_value_path_during_inference.scala:16:10: error: cannot resolve export: pin not found
// expect: 1 error found
// absent: recursive value
// The export table a type lookup builds while a val's type is inferred leaves a clause through a
// value to the value forwarders: `Foo` in `foo`'s initializer is looked up in `Test`, whose
// `export foo.pin.*` would widen `foo` while it is inferred. dotty's class completer sees `Foo`
// before the forwarders; only the export is in error, `foo.pin` needing reflection. scalac's
// error is E083, `(Test.foo.pin : Object) is not a valid export prefix, since it is not an
// immutable path`, over `foo`'s type `Foo{val pin: Object}`: teq types `foo` as `Foo`, keeping no
// refinement of an anonymous class's new members, and finds no `pin`.
trait Foo extends reflect.Selectable
object Test:
  val foo = new Foo:
    object pin:
      val x = 1
  export foo.pin.*

object OK:
  object Foo:
    object pin:
      val x = 1
  export Foo.pin.*
