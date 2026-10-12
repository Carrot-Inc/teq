// An enum with a secondary constructor that its cases call, a case naming another case in its
// parent's arguments (dotty's checkEnumCaseRefsLegal imports the cases there), and a secondary
// constructor with statements after its self call.
enum Perm(val key: String, val also: List[Perm], val name: String):
  def this(key: String, name: String) = this(key, Nil, name)
  def this(key: String) =
    this(key, key.toLowerCase)
    println(s"made $key")
  case A extends Perm("A", "a")
  case B extends Perm("B", List(A), "b")
  case C extends Perm("C")
  case D extends Perm("D", List(B, C), "d")

@main def run(): Unit =
  for p <- Perm.values do println((p, p.key, p.also, p.name))
  println(Perm.D.also.flatMap(_.also))
