object UseCopiedCalls:
  def size(x: (Int, String)): Int = CopiedCalls.size(x)
  def built(x: Int): Int = CopiedCalls.built(x)
