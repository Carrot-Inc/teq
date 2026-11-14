object Part12:
  def pad12(n: Int): Int = n + 12
  def run(n: Int): Int =
    def first: Int = second + pad12(n)
    val held: Int = first
    def second: Int = n
    held
