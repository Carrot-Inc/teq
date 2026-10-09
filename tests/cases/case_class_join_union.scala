// The join of two unrelated case classes is transparent (`Product & Serializable`), so a
// val's inferred type keeps their union, as dotty's `ConstraintHandling.widenInferred` does.
final case class Tag(name: String)
case object EmptyNode
case class Other(x: Int)

def pickOther(x: Product): String = "product"
def pickOther(x: Tag | Other): String = "tag or other"
def pickEmpty(x: Product): String = "product"
def pickEmpty(x: Tag | EmptyNode.type): String = "tag or empty"
def pickPair(x: Product): String = "product"
def pickPair(x: Some[Int] | (Int, Int)): String = "some or pair"

@main def main(): Unit =
  val c = "x".length == 1
  val n = if c then Tag("a") else EmptyNode
  println(pickEmpty(n))
  val o = if c then Tag("a") else Other(1)
  println(pickOther(o))
  val q = List(Tag("a"), Other(1))
  println(pickOther(q.head))
  val s = if c then Some(1) else (1, 2)
  println(pickPair(s))
