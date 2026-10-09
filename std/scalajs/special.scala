// Scala.js's `js.special`: the JavaScript operators that have no method syntax.
package scala.scalajs.js.special

@js("delete $0[$1]")
def delete(obj: scala.Any, key: scala.Any): Unit

@js("($0 in $1)")
def in(key: scala.Any, obj: scala.Any): Boolean

@js("($0 instanceof $1)")
def instanceof(x: scala.Any, constructor: scala.Any): Boolean

@js("($0 === $1)")
def strictEquals(x: scala.Any, y: scala.Any): Boolean

@js("debugger")
def debugger(): Unit
