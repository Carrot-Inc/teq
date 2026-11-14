object Part03:
  def pad3(n: Int): Int = n + 3
  def run(n: Int): Int =
    def first: Int = second + pad3(n)
    val held: Int = first
    def second: Int = n
    held
