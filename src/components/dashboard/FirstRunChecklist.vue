<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Icon } from '@iconify/vue';

const { t } = useI18n({
  useScope: 'local',
  messages: {
    en: {
      checklist: {
        title: 'Companion setup checklist',
        hidePermanently: 'Hide permanently',
        dismiss: 'Dismiss',
        startWithSystem: {
          title: 'Start with system',
          desc: 'Turn on the auto-start toggle in Settings → General',
        },
        firewall: {
          title: 'Firewall / Port Rule',
          desc: 'Allow the WebSocket port through Windows Defender Firewall',
        },
        webClient: {
          title: 'Web Client (iPad)',
          desc: 'Enable Web Client in Settings if using a browser on iPad',
        },
        scanQr: {
          title: 'Scan QR to download APK',
          desc: 'Scan the QR code below to download the APK, or open the Web URL on your Android device',
        },
      },
    },
    vi: {
      checklist: {
        title: 'Checklist cài đặt Companion',
        hidePermanently: 'Ẩn vĩnh viễn',
        dismiss: 'Dismiss',
        startWithSystem: {
          title: 'Khởi động cùng hệ thống',
          desc: 'Bật toggle tự động khởi động trong Settings → General',
        },
        firewall: {
          title: 'Firewall / Port Rule',
          desc: 'Cho phép cổng WebSocket qua Windows Defender Firewall',
        },
        webClient: {
          title: 'Web Client (iPad)',
          desc: 'Bật Web Client trong Settings nếu dùng trình duyệt trên iPad',
        },
        scanQr: {
          title: 'Quét QR tải APK',
          desc: 'Quét QR bên dưới để tải APK hoặc mở Web URL trên thiết bị Android',
        },
      },
    },
  },
});

defineProps<{
  firstRunDismissed: boolean;
}>();

const emit = defineEmits<{
  (e: 'dismiss'): void;
}>();
</script>

<template>
  <transition name="fade">
    <div
      v-if="!firstRunDismissed"
      class="cyber-panel flex flex-col gap-3 px-4 py-3 border-cyan-400/30"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Icon icon="lucide:clipboard-list" class="text-base text-cyan-400 shrink-0" />
          <span class="text-[11px] font-bold text-cyan-300 uppercase tracking-wider"
            >{{ t('checklist.title') }}</span
          >
        </div>
        <button
          type="button"
          class="cyber-action-btn font-bold cursor-pointer text-[10px] uppercase tracking-wider px-3 py-1 flex items-center gap-1.5"
          @click="emit('dismiss')"
          :title="t('checklist.hidePermanently')"
        >
          <Icon icon="lucide:x" class="text-xs" />
          <span>{{ t('checklist.dismiss') }}</span>
        </button>
      </div>
      <div class="grid grid-cols-2 gap-2 xl:grid-cols-4">
        <div class="cyber-inset flex items-start gap-2 p-3">
          <Icon icon="lucide:power" class="text-sm text-cyan-400 shrink-0 mt-0.5" />
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] font-bold text-slate-200">{{ t('checklist.startWithSystem.title') }}</span>
            <span class="text-[9px] text-slate-500 leading-relaxed"
              >{{ t('checklist.startWithSystem.desc') }}</span
            >
          </div>
        </div>
        <div class="cyber-inset flex items-start gap-2 p-3">
          <Icon icon="lucide:shield" class="text-sm text-amber-400 shrink-0 mt-0.5" />
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] font-bold text-slate-200">{{ t('checklist.firewall.title') }}</span>
            <span class="text-[9px] text-slate-500 leading-relaxed"
              >{{ t('checklist.firewall.desc') }}</span
            >
          </div>
        </div>
        <div class="cyber-inset flex items-start gap-2 p-3">
          <Icon icon="lucide:globe" class="text-sm text-fuchsia-400 shrink-0 mt-0.5" />
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] font-bold text-slate-200">{{ t('checklist.webClient.title') }}</span>
            <span class="text-[9px] text-slate-500 leading-relaxed"
              >{{ t('checklist.webClient.desc') }}</span
            >
          </div>
        </div>
        <div class="cyber-inset flex items-start gap-2 p-3">
          <Icon icon="lucide:qr-code" class="text-sm text-emerald-400 shrink-0 mt-0.5" />
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] font-bold text-slate-200">{{ t('checklist.scanQr.title') }}</span>
            <span class="text-[9px] text-slate-500 leading-relaxed"
              >{{ t('checklist.scanQr.desc') }}</span
            >
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>
