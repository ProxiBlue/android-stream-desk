<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Icon } from '@iconify/vue';

const { t } = useI18n({
  useScope: 'local',
  messages: {
    en: {
      qrZoom: {
        zoomInAriaLabel: 'Zoom in {title}',
        scanBankingApp: 'Scan the code with your banking app / e-wallet',
        scanQrCode: 'Scan the QR code with your phone or iPad camera',
        closeModal: 'Close modal',
        connectionLink: 'Connection link',
        copy: 'Copy',
      },
    },
    vi: {
      qrZoom: {
        zoomInAriaLabel: 'Phóng to {title}',
        scanBankingApp: 'Quét mã bằng app ngân hàng / ví điện tử',
        scanQrCode: 'Quét mã QR từ camera điện thoại hoặc iPad',
        closeModal: 'Đóng modal',
        connectionLink: 'Đường dẫn kết nối',
        copy: 'Sao chép',
      },
    },
  },
});

defineProps<{
  modelValue: boolean;
  zoomModalTitle: string;
  zoomModalImageSrc: string;
  zoomModalQrSvg: string;
  zoomModalPayload: string;
  zoomModalCopyHint: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'copy'): void;
}>();
</script>

<template>
  <transition name="fade">
    <div
      v-if="modelValue"
      class="fixed inset-0 z-[70] flex items-center justify-center bg-black/85 backdrop-blur-sm p-4 cursor-default"
      @click.self="emit('update:modelValue', false)"
    >
      <div
        class="bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full flex flex-col p-6 gap-5 shadow-2xl relative overflow-hidden"
        role="dialog"
        aria-modal="true"
        :aria-label="t('qrZoom.zoomInAriaLabel', { title: zoomModalTitle })"
      >
        <!-- Header -->
        <div class="flex items-center justify-between">
          <div class="flex flex-col gap-0.5">
            <h3 class="text-xs font-bold text-slate-100 uppercase tracking-widest">
              {{ zoomModalTitle }}
            </h3>
            <p class="text-[9.5px] text-slate-450">
              {{
                zoomModalImageSrc
                  ? t('qrZoom.scanBankingApp')
                  : t('qrZoom.scanQrCode')
              }}
            </p>
          </div>
          <button
            type="button"
            class="w-6 h-6 rounded-md hover:bg-slate-800 flex items-center justify-center text-slate-400 hover:text-slate-200 transition-colors cursor-pointer"
            @click="emit('update:modelValue', false)"
            :title="t('qrZoom.closeModal')"
          >
            <Icon icon="lucide:x" class="text-sm" />
          </button>
        </div>

        <!-- Plain White QR Container (no glow/filter) -->
        <div
          class="flex justify-center py-2 bg-slate-950/40 rounded-xl p-4 border border-slate-850"
        >
          <div
            class="w-full max-w-96 aspect-square bg-white p-2.5 rounded-xl shadow-[0_4px_24px_rgba(0,0,0,0.4)] transition-transform overflow-hidden"
          >
            <img
              v-if="zoomModalImageSrc"
              :src="zoomModalImageSrc"
              :alt="zoomModalTitle"
              class="w-full h-full object-contain"
            />
            <div v-else v-html="zoomModalQrSvg" class="w-full h-full"></div>
          </div>
        </div>

        <!-- Payload copy & detail -->
        <div v-if="!zoomModalImageSrc" class="flex flex-col gap-2">
          <div
            class="flex items-center justify-between text-[9px] uppercase tracking-wider text-slate-450 font-bold px-1"
          >
            <span>{{ t('qrZoom.connectionLink') }}</span>
            <button
              type="button"
              class="hover:text-cyan-400 flex items-center gap-1 cursor-pointer transition-colors"
              @click="emit('copy')"
            >
              <Icon
                :icon="zoomModalCopyHint ? 'lucide:check' : 'lucide:copy'"
                class="text-[10px]"
              />
              <span>{{ zoomModalCopyHint || t('qrZoom.copy') }}</span>
            </button>
          </div>
          <div
            class="rounded-lg border border-slate-800 bg-slate-950/70 px-3 py-2 font-mono text-[10.5px] text-slate-350 shadow-inner break-all max-h-24 overflow-y-auto selection:bg-cyan-550/30 select-text"
          >
            {{ zoomModalPayload }}
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>
