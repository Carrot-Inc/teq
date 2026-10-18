object A { import scala.collection.immutable.Map; type M = Int Map String }
object B { import scala.collection.immutable.*; type M = Int Map String }
object C { type M = Int Map String }
object D { import scala.*; type M = Int Map String }
object E { import scala.{Either as Ei}; type M = Int Ei String }
object F { type P = Int PartialFunction String }
