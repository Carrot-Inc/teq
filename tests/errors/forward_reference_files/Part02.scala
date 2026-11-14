object Part02:
  def pad2(n: Int): Int = n + 2
  def run(n: Int): Int =
    def first: Int = second + pad2(n)
    val held: Int = first
    def second: Int = n
    held
