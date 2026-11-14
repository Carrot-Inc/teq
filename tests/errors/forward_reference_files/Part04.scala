object Part04:
  def pad4(n: Int): Int = n + 4
  def run(n: Int): Int =
    def first: Int = second + pad4(n)
    val held: Int = first
    def second: Int = n
    held
