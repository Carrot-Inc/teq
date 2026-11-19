object O12 { extension (i: Int) def call(using need: Numeric[String]): Int = i }

import O12.*
val result12 = 1.call
// scalac 3.8.4 tells an extension that applies to no receiver's member as E008 (`value call is not a member
// of Int.`, the extension's failure under it); teq gives the extension's missing given (E172): another family, a row.
// expect: c12.scala:4:18: error: value call is not a member of Int.
// teq: --std=scala-library
// expect: 3 errors found
