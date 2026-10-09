package scala.scalajs.js.timers

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobalScope
import scala.concurrent.duration.FiniteDuration

/** The value the browser's setTimeout returns (a number; an object under node). */
@js.native
trait SetTimeoutHandle extends js.Object

@js.native
trait SetIntervalHandle extends js.Object

@js.native
@JSGlobalScope
object RawTimers extends js.Object:
  def setTimeout(handler: js.Function0[scala.Any], interval: Double): SetTimeoutHandle = js.native
  def clearTimeout(handle: SetTimeoutHandle): Unit = js.native
  def setInterval(handler: js.Function0[scala.Any], interval: Double): SetIntervalHandle = js.native
  def clearInterval(handle: SetIntervalHandle): Unit = js.native

def setTimeout(interval: Double)(body: => Unit): SetTimeoutHandle =
  RawTimers.setTimeout(() => body, interval)

def setTimeout(interval: FiniteDuration)(body: => Unit): SetTimeoutHandle =
  RawTimers.setTimeout(() => body, interval.toMillis.toDouble)

def clearTimeout(handle: SetTimeoutHandle): Unit = RawTimers.clearTimeout(handle)

def setInterval(interval: Double)(body: => Unit): SetIntervalHandle =
  RawTimers.setInterval(() => body, interval)

def setInterval(interval: FiniteDuration)(body: => Unit): SetIntervalHandle =
  RawTimers.setInterval(() => body, interval.toMillis.toDouble)

def clearInterval(handle: SetIntervalHandle): Unit = RawTimers.clearInterval(handle)
