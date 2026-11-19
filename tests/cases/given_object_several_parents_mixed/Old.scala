package pk
package inner

import scala.language.`3.6`

object Old:
  def get: String = s"${summon[P].x}${summon[Q].q}"
