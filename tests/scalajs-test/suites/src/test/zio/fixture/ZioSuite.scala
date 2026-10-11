package fixture

import zio.test.*

object ZioSuite extends ZIOSpecDefault:
  def spec = suite("zio")(
    test("passes")(assertTrue(Core.twice(6) == 12)),
    test("fails")(assertTrue(Core.twice(6) == 13)),
    test("is ignored")(assertTrue(false)) @@ TestAspect.ignore,
  )
