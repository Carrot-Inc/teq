// jars: hiddenowner-lib
// A Java class's statics are members of its companion alone, as dotty's `ClassfileParser` enters them:
// the ones a public class inherits from a package-private one (hidden.Base's, through hidden.Shown)
// or from a public one (hidden.StaticBase's, through hidden.StaticSub) are not members of the
// subclass's companion, read or written, which scalac 3.8.4 refuses too ("value staticMethod is not a
// member of object hidden.Shown"). tests/classpath/jvm/hidden_owner.scala reaches the declaring
// class's statics.
// expect: 19:30: error: value staticMethod is not a member of Shown
// expect: 20:27: error: value staticField is not a member of Shown
// expect: 21:29: error: value staticField is not a member of Shown
// expect: 22:30: error: value staticMethod is not a member of StaticSub
// expect: 23:27: error: value staticField is not a member of StaticSub
// expect: 24:29: error: value staticField is not a member of StaticSub
package ownerstatics

import hidden.{Shown, StaticSub}

object Statics:
  def throughHidden(): Int = Shown.staticMethod()
  def readHidden(): Int = Shown.staticField
  def writeHidden(): Unit = Shown.staticField = 10
  def throughPublic(): Int = StaticSub.staticMethod()
  def readPublic(): Int = StaticSub.staticField
  def writePublic(): Unit = StaticSub.staticField = 10
