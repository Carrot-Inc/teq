package meetmin
object Both extends A with B with C { type T = String }
object Main:
  def main(args: Array[String]): Unit =
    println(Api.two(Both)("ok"))
    println(Api.three(Both)("ok"))
    println(Api.twoResult(Both)("two"))
    println(Api.threeResult(Both)("three"))
    println(Api.twoArray(Both)(Array("two-array"))(0))
    println(Api.threeArray(Both)(Array("three-array"))(0))
