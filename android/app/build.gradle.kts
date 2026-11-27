plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "app.streamphone"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "app.streamphone"
        // Android 8: the oldest phones the dashboard itself supports.
        minSdk = 26
        targetSdk = 37
        versionCode = 10400
        versionName = "1.4.0"
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Signed with the local debug key until the project has a release key;
            // see README ("Signing"). Unlike a debug build it is not debuggable.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    // The QR scanner for pairing over Wi-Fi.
    implementation(libs.zxing.embedded)
    // The scanner asks for the camera permission through AndroidX, which it
    // does not bring along itself; without it the app closes when scanning.
    implementation(libs.androidx.core)
}
