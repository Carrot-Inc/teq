package ja

import java.util.UUID

object Ids:
  def fixed: UUID = UUID.fromString("00000000-0000-0000-0000-000000000001")
  def text(id: UUID): String = id.toString
