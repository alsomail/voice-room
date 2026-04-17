import java.util.Properties

plugins {
    id("com.android.application")
    kotlin("android")
    id("org.jetbrains.kotlinx.kover")
}

fun loadLocalProperties(rootDir: File): Properties {
    val properties = Properties()
    val localPropertiesFile = rootDir.resolve("local.properties")

    if (localPropertiesFile.exists()) {
        localPropertiesFile.inputStream().use(properties::load)
    }

    return properties
}

fun resolveConfigValue(
    localProperties: Properties,
    propertyName: String,
    envName: String,
    defaultValue: String
): String = localProperties.getProperty(propertyName)
    ?: System.getenv(envName)
    ?: defaultValue

val localProperties = loadLocalProperties(rootProject.projectDir)

android {
    namespace = "com.voice.room.android"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.voice.room.android"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        buildConfigField(
            "String",
            "APP_ENVIRONMENT",
            "\"${resolveConfigValue(localProperties, "voiceRoomEnvironment", "VOICE_ROOM_ENVIRONMENT", "dev")}\""
        )
        buildConfigField(
            "String",
            "API_BASE_URL",
            "\"${resolveConfigValue(localProperties, "voiceRoomApiBaseUrl", "VOICE_ROOM_API_BASE_URL", "https://dev-api.example.com/api")}\""
        )
        buildConfigField(
            "String",
            "WS_URL",
            "\"${resolveConfigValue(localProperties, "voiceRoomWsUrl", "VOICE_ROOM_WS_URL", "wss://dev-api.example.com/ws")}\""
        )
        buildConfigField(
            "String",
            "ANALYTICS_ENDPOINT",
            "\"${resolveConfigValue(localProperties, "voiceRoomAnalyticsEndpoint", "VOICE_ROOM_ANALYTICS_ENDPOINT", "https://analytics-dev.example.com/collect")}\""
        )
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
            manifestPlaceholders["usesCleartextTraffic"] = "true"
        }

        release {
            isMinifyEnabled = false
            manifestPlaceholders["usesCleartextTraffic"] = "false"
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    testOptions {
        unitTests.isIncludeAndroidResources = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.activity:activity-ktx:1.9.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.8.6")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    testImplementation("junit:junit:4.13.2")

    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.6.1")
    androidTestImplementation("androidx.test:rules:1.6.1")
}

kover {
    reports {
        filters {
            excludes {
                classes(
                    "com.voice.room.android.VoiceRoomApplication",
                    "com.voice.room.android.presentation.MainActivity*",
                    "*.BuildConfig",
                    "*.R",
                    "*.R$*"
                )
            }
        }
    }
}
