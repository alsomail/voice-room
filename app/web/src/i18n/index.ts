/**
 * i18n 配置入口
 * 支持语言：en（英文）、zh（中文）
 * 默认语言：en
 *
 * 使用方式：
 *   import './i18n';          // 在 main.tsx 中引入一次即可
 *   const { t } = useTranslation();
 */
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './locales/en';
import zh from './locales/zh';

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    zh: { translation: zh },
  },
  lng: 'zh',
  fallbackLng: 'zh',
  interpolation: {
    escapeValue: false, // React 已自动转义 XSS
  },
});

export default i18n;
