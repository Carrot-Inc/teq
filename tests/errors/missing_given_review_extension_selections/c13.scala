trait X13 { extension (i: Int) def call(using need: Numeric[String]): Int = i }
object O13 { given gx: X13 = new X13 {} }

import O13.gx.*
val result13 = 1.call
// scalac 3.8.4 tells an extension that applies to no receiver's member as E008 (`value call is not a member
// of Int.`, the extension's failure under it); teq gives the extension's missing given (E172): another family, a row.
// expect: c13.scala:5:18: error: value call is not a member of Int.
