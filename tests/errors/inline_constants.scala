// expect: 13:18: error: inline value must have a literal constant type
// expect: 14:18: error: inline value must have a literal constant type
// expect: 15:21: error: inline value must have a literal constant type
// expect: 16:21: error: inline value must have a literal constant type
// expect: 17:21: error: inline value must have a literal constant type
// expect: 18:21: error: inline value must have a literal constant type
// expect: 19:21: error: inline value must have a literal constant type
// expect: 20:21: error: inline value must have a literal constant type
// expect: 21:21: error: inline value must contain a literal constant value.
// expect: 9 errors found
object Constants:
  final val k = 2
  inline val n = "ab".length
  inline val s = "x" + k
  inline val neg1 = math.max(1, 2)
  inline val neg2 = { val q = 1; q + 1 }
  inline val neg3 = (1, 2)._1
  inline val neg4 = "ab".toUpperCase
  inline val neg5 = s"a${"b"}"
  inline val neg6 = k > 1 && "ab".length > 1
  inline val neg7 = Some(1)
  inline val ok1 = 1 + k * 3
  inline val ok2 = "a" + "b"
  inline val ok3 = -ok1 + 1
  inline val ok4 = ok2 == "ab"
  inline val ok5 = 'c'.toInt + 1L
  inline val ok6 = 1.5 * 2
