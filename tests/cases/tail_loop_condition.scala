object Main {
  def check(n: Int, limit: Int): Boolean = {
    println(n)
    return if n % 2 == 0 then n < limit && check(n + 1, limit)
           else n == limit || check(n + 1, limit)
  }
  def main(args: Array[String]): Unit = {
    println(check(0, 3))
    println(check(1, 4))
  }
}
