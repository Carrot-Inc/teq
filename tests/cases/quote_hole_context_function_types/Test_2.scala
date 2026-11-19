@main def main(): Unit =
  println(M.show(Api.one(summon[Int] + 1)))
  println(M.show(Api.two(summon[Int] + summon[String].length)))
  println(M.show(Api.byName(summon[Int])))
  println(M.show(Api.aliased(summon[Int])))
  println(M.show(Api.plain(x => x + 1)))
  println(M.show(Api.outer(Api.one(summon[Int]))))
