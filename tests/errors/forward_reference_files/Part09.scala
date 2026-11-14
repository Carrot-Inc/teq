object Part09:
  def pad9(n: Int): Int = n + 9
  def run(n: Int): Int =
    def first: Int = second + pad9(n)
    val held: Int = first
    def second: Int = n
    held
