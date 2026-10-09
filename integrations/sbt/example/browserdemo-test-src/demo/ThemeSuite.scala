package demo

import demo.model.{Theme, Themes}

class ThemeSuite extends munit.FunSuite:
  test("the light theme turns dark") {
    assertEquals(Themes.next(Theme.Light), Theme.Dark(1))
  }

  test("two steps from light") {
    assertEquals(Themes.next(Themes.next(Theme.Light)), Theme.Dark(2))
  }
