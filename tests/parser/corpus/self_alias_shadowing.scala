// A `self =>` alias is a name like any other: a parameter, a lambda parameter or a local val
// called `self` shadows it, and a nested class's own alias shadows the enclosing one's.
trait Named { def name: String }

class Node(val name: String):
  self =>
  def viaAlias: String = self.name
  def viaParam(self: Node): String = self.name
  def viaLambda: List[String] = List(Node("a"), Node("b")).map(self => self.name)
  def viaLocal: String =
    val self = "local"
    self
  def nested: Named = new Named:
    self =>
    def name = "inner of " + Node.this.name
    override def toString = self.name
  def outerFromLambda: List[String] = List(1, 2).map(i => self.name + i)
  type Me = self.type
  def me: Me = self

@main def Main(): Unit =
  val n = Node("n")
  println(n.viaAlias)
  println(n.viaParam(Node("p")))
  println(n.viaLambda)
  println(n.viaLocal)
  println(n.nested)
  println(n.outerFromLambda)
  println(n.me.name)
