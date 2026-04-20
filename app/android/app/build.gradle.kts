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
        compose = true
    }

    composeOptions {
        // Kotlin 1.9.24 → Compose Compiler 1.5.14
        kotlinCompilerExtensionVersion = "1.5.14"
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
        // 让 android.util.Log 等 Android 框架方法在 JVM 单测中返回默认值（0/null），
        // 不抛出 "Method not mocked" RuntimeException
        unitTests.isReturnDefaultValues = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.activity:activity-ktx:1.9.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.8.6")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // Jetpack Compose BOM – pins all Compose versions consistently
    val composeBom = platform("androidx.compose:compose-bom:2024.09.00")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.6")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.6")

    // Navigation Compose — 全局导航骨架 (T-30019)
    implementation("androidx.navigation:navigation-compose:2.8.1")

    // Coil – 异步图片加载（房主头像）
    implementation("io.coil-kt:coil-compose:2.6.0")

    // Coroutines (needed for StateFlow in ViewModel)
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")

    // Retrofit + Gson converter
    implementation("com.squareup.retrofit2:retrofit:2.11.0")
    implementation("com.squareup.retrofit2:converter-gson:2.11.0")

    // DataStore Preferences (JWT token storage)
    implementation("androidx.datastore:datastore-preferences:1.1.1")

    // Paging3 — 无限滚动、下拉刷新 (T-30006)
    implementation("androidx.paging:paging-runtime:3.2.1")
    implementation("androidx.paging:paging-compose:3.2.1")

    // Accompanist Permissions — 运行时权限请求 (T-30012)
    implementation("com.google.accompanist:accompanist-permissions:0.36.0")

    // Unit tests
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")
    testImplementation("androidx.paging:paging-testing:3.2.1")
    testImplementation("com.squareup.okhttp3:mockwebserver:4.12.0")

    // Android instrumented tests
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.6.1")
    androidTestImplementation("androidx.test:rules:1.6.1")
    androidTestImplementation(platform("androidx.compose:compose-bom:2024.09.00"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.paging:paging-testing:3.2.1")

    // Compose UI debug tooling
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
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
                    "*.R$*",
                    // Compose-generated lambda classes (covered by androidTest)
                    "*.*ScreenKt*",
                    "*.ComposableSingletons*"
                )
            }
        }
    }
}
