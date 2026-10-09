// expect: 14:19: error: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 15:22: error: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 16:23: error: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 17:22: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// absent: warning: the type test for
// The part of a generator's pattern that can fail is reported with its own type against the
// scrutinee type of its place, there (dotty's `Checking.checkIrrefutable`, whose recursion carries
// each component's scrutinee type): a nested extractor, a typed binder at its type, a pattern two
// levels down; a pattern val warns at its right-hand side.
@main def run(): Unit =
  val m: Map[Int, Option[Int]] = Map(1 -> Some(2))
  val xs: List[(Int, (String, Option[Int]))] = List((1, ("a", Some(2))))
  val p: (Int, Option[Int]) = (1, Some(2))
  val a = for (k, Some(v)) <- m yield k + v
  val b = for (k, v: Some[Int]) <- m yield k + v.value
  val c = for (k, (s, Some(v))) <- xs yield k + v
  val (k, Some(v)) = p
  println((a, b, c, k, v))
