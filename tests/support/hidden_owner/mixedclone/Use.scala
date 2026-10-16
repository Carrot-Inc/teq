// The class mixing in tests/support/hidden_owner/mixedclone's traits, built by teq over the traits teq built
// and over scalac's.
package mixedcloneuse

import mixedclone.*

class E extends C with G

object Main:
  def main(args: Array[String]): Unit = println((new E: C).clone())
