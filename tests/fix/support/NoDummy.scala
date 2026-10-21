// The test rule NoDummy of scalafix's escape-hatch fixtures (https://github.com/scalacenter/scalafix, commit
// 41cf7983a19a0453be78d1fe9863d77470ca6dec, scalafix-tests/integration/src/main/scala/scalafix/test/NoDummy.scala;
// BSD 3-Clause License, Copyright (c) 2016-2023, EPFL, NOTICE), ported from scalafix's v0 API to teq's: an error at
// each name holding `dummy`, whatever its case, with the category's explanation as its message.
package scalafix.test

import teq.fix.*

final class NoDummy extends SyntacticRule("NoDummy"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case tree @ Name(name) if name.toLowerCase.contains("dummy") => Patch.lint(Diagnostic("", "Dummy!", tree.pos))
    }.asPatch
