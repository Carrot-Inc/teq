package demo

import zio.test.*

import demo.model.{Theme, Themes}

object ThemeSpec extends ZIOSpecDefault:
  def spec = suite("Themes")(
    test("the light theme turns dark") {
      assertTrue(Themes.next(Theme.Light) == Theme.Dark(1))
    },
    test("a dark theme deepens") {
      assertTrue(Themes.next(Theme.Dark(2)) == Theme.Dark(3))
    },
  )
