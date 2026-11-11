package vca

// A value class's erased value is its underlying type's erasure: `(U1 | U2) & Z`, whose `Pure` side's class symbol
// reads the join of `U1 | U2` (TypeErasure.scala:801-804), so a method of another module that takes a `W` takes `Z`.
// The typer keeps that join for the emitter wherever `W` comes from, this module's sources or its products.
import scala.caps.Pure
trait Z
trait U1 extends Any with Pure
trait U2 extends Any with Pure
class ZU1 extends Z with U1
class W(val a: (U1 | U2) & Z) extends AnyVal
