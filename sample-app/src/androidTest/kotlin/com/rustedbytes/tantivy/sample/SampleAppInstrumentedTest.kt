package com.rustedbytes.tantivy.sample

import androidx.test.core.app.ActivityScenario
import kotlin.test.Test
import kotlin.test.assertNotNull

class SampleAppInstrumentedTest {
    @Test
    fun launchesMainActivity() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { activity ->
                assertNotNull(activity)
            }
        }
    }
}
