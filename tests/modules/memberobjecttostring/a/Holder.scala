// The program's `toString` of an object nested in an instance (`h.In`, of a type nested in the instance's class) is
// written as its class's, a Scala class's: the conversion of a Scala instance, not Scala.js's dispatch of a value
// that may be JavaScript's, in the whole bundle and in the one built over the products alike.
package mota

class Holder:
  object In:
    override def toString: String = "in"
  object Plain
