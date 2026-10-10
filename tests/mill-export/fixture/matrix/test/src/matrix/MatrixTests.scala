package matrix

import utest.*

object MatrixTests extends TestSuite:
  val tests = Tests {
    test("twice") {
      assert(Matrix.twice(21) == 42)
    }
  }
