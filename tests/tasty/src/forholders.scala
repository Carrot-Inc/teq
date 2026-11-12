package fix.forh

// The holders after a `for` take their numbers after every name `makeFor` takes (`ForGroups`,                   
// `ForNested`): each run of aliases after a guard binds the fallback's generator over the tuple                        
// of the written patterns to a fresh name (`makeIdPat(Tuple(allpats))`), and an alias of a nested given pattern goes
// the fallback's way under 3.8 (`isNestedGivenPattern`). The object after each takes the next number.
object ForGroups {
  def f = for {
    x <- List(1)
    a = x
    if a > 0
    b = x
    if b > 0
    (c, d) = (1, 2)
    if c > 0
  } yield x + a + b + c + d
}
object AfterGroups {
  val List(a, b) = List(1, 2): @unchecked
}
object ForNested {
  def f = for {
    (x, y) <- List((1, 2))
    (a, given Int) = (3, 4)
  } yield x + y + a + summon[Int]
}
object AfterNested {
  val List(a, b) = List(1, 2): @unchecked
}
