package demo

import zio.test.*

import demo.model.{Theme, Themes}

object FailingThemeSpec extends ZIOSpecDefault:
  def spec = suite("Themes, wrongly")(
    test("the light theme stays light") {
      assertTrue(Themes.next(Theme.Light) == Theme.Light)
    },
  )
