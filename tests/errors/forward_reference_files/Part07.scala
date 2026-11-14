object Part07:
  def pad7(n: Int): Int = n + 7
  def run(n: Int): Int =
    def first: Int = second + pad7(n)
    val held: Int = first
    def second: Int = n
    held
