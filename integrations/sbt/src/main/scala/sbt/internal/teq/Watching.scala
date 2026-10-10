package sbt.internal.teq

import sbt.{StandardMain, State}
import sbt.internal.ContinuousCommands
import sbt.nio.Watch

/** What sbt keeps to itself about a watch (`~`), for the link tasks' residents. */
object Watching {
  /** The watch that the command under way is an iteration of, by the name of its channel: sbt
    * queues the watch's own command behind every command of an iteration. */
  def of(state: State): Option[String] =
    state.remainingCommands.map(_.commandLine).collectFirst {
      case line if line.startsWith(ContinuousCommands.postWatch) => line.stripPrefix(ContinuousCommands.postWatch).trim
    }

  /** Whether sbt has a watch under way on the channel, by the state its command loop had when
    * it last waited for a command, which is where a watch waits for a change. */
  def underWay(channel: String): Boolean = {
    val exchange = StandardMain.exchange
    exchange.channelForName(channel).exists(c => exchange.withState(state => state != null && ContinuousCommands.isInWatch(state, c)))
  }

  /** sbt's hook for the end of a watched task where a build defines none. */
  val defaultOnTermination: (Watch.Action, String, Int, State) => State = Watch.defaultTaskOnTermination
}
