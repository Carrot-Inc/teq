package ica

// An object constructing a class it inherits from a trait: the `new`'s type is the class through the
// object's `this` (`Derived.this.One`, as `Typer.newExpr` types it and scalac pickles it), which a
// downstream reads back from the products and scalac's tree checker takes.
trait Scope { class One(val n: Int) }
object Derived extends Scope { val one = new One(1) }
