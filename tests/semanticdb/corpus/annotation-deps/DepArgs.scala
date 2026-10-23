object DepArgs {
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val direct = map(new C)(x => (??? : Box[x.T @Ann(x)]))
  val block = map(new C)(x => (??? : Box[x.T @Ann({ val y = x; 1 })]))
  val cases = map(new C)(x => (??? : Box[x.T @Ann(1 match { case _ => x })]))
  val attempt = map(new C)(x => (??? : Box[x.T @Ann(try x finally ())]))
}
