// jars: scala-java-time scala-java-time-tzdb
// The tzdb registration from a program whose own package shares its first segment with a
// package of the std's Scala.js layer (`org.portablescala.reflect`, which scala-java-time's
// default initializer names): the layer still opens when the jar's body reaches it. The
// expectation is the JDK's for the same lines without the registration.
package org.example

import java.time.*
import java.time.zone.{ZoneRulesInitializer, ZoneRulesProvider, TzdbZoneRulesProvider}

object Main:
  def main(args: Array[String]): Unit =
    ZoneRulesInitializer.setInitializer(new ZoneRulesInitializer:
      def initializeProviders(): Unit = ZoneRulesProvider.registerProvider(new TzdbZoneRulesProvider()))
    val i = Instant.ofEpochSecond(1709214330L)
    println(i.atZone(ZoneId.of("America/New_York")))
    println(ZoneId.of("Europe/Berlin").getRules.getOffset(i))
