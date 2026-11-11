// A class nested in a class is applied through a path (`o.In[...]` of `val o = new Outer[Int]`): its parameter's
// bound `X` is seen from the prefix (`paramInfoAsSeenFrom(tree.tpe)`, Checking.scala 118), `Int`, so `o.In[? >: Int]`
// overlaps it and scalac 3.8.4 accepts it (and rejects `o.In[? >: String]`, `upper bound Int`). teq sees the type
// `o.In[? >: Int]` from its prefix since round 22 (`check_type_args_through`'s `seen`), but not the class that `new`
// instantiates, whose bound it reads unseen, as the outer parameter `X`:
//   12:14: error: type argument Int does not conform to upper bound X
class Outer[X]:
  class In[T <: X](val n: Int)
val o = new Outer[Int]
type B = o.In[? >: Int]
@main def main(): Unit =
  val b: B = new o.In[Int](5)
  println(b.n)
