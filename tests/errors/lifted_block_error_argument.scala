object Sample:
  def keep(value: Int): Int = value
  def pair(first: Int, second: Int): Int = first + second
  val rejected = absent(pair(second = keep({ val n = 2; n }), first = keep({ val n = 1; n })))
// expect: 4:18: error: not found: absent
// expect: 1 error found
