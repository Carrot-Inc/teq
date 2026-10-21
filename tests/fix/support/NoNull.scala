// The test rule NoNull of scalafix's escape-hatch fixtures (https://github.com/scalacenter/scalafix, commit
// 41cf7983a19a0453be78d1fe9863d77470ca6dec, scalafix-tests/integration/src/main/scala/scalafix/test/NoNull.scala;
// BSD 3-Clause License, Copyright (c) 2016-2023, EPFL, NOTICE), ported from scalafix's v0 API to teq's.
package scalafix.test

import teq.fix.*

final class NoNull extends SyntacticRule("NoNull"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case nil: Lit.Null => Patch.lint(Diagnostic("", "Nulls are not allowed.", nil.pos))
    }.asPatch
