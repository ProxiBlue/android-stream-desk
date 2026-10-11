<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Icon } from '@iconify/vue';

const { t } = useI18n({
  useScope: 'local',
  messages: {
    en: {
      recovery: {
        title: 'Recover Accessibility permission',
        bundleIdentifierFallback: 'Bundle identifier',
        copiedBundleId: 'Copied bundle id',
        bundleIdUnknown: 'bundle id unknown',
        resetSteps:
          'Dev build reset steps: quit app → remove old entry → drag the correct `.app` into Accessibility → turn it back on → open app → check again.',
        copyExecutablePath: 'Copy executablePath',
        couldNotResolve: 'Could not resolve',
        devBinary: 'Dev binary',
        copyAppBundlePath: 'Copy appBundlePath',
        openSettings: 'Open Accessibility Settings',
        recheckTitle: 'Recheck permission',
        recheck: 'Recheck',
      },
    },
    vi: {
      recovery: {
        title: 'Khôi phục Accessibility permission',
        bundleIdentifierFallback: 'Mã bundle',
        copiedBundleId: 'Đã copy bundle id',
        bundleIdUnknown: 'bundle id chưa rõ',
        resetSteps:
          'Quy trình reset dev build: quit app → xoá entry cũ → kéo đúng `.app` vào Accessibility → bật lại → mở app → kiểm tra lại.',
        copyExecutablePath: 'Sao chép executablePath',
        couldNotResolve: 'Không resolve được',
        devBinary: 'Dev binary',
        copyAppBundlePath: 'Sao chép appBundlePath',
        openSettings: 'Mở Accessibility Settings',
        recheckTitle: 'Kiểm tra lại quyền',
        recheck: 'Kiểm tra lại',
      },
    },
  },
});

defineProps<{
  showAccessibilityRecovery: boolean;
  inputPermissionDiagnostics: {
    trusted: boolean;
    bundleIdentifier: string;
    executablePath: string | null;
    appBundlePath: string | null;
    isPackagedApp: boolean;
    recommendedAction: string;
  } | null;
  shortBundleIdentifier: string;
  inputPermissionActionText: string;
  permissionCopyHint: string;
}>();

const emit = defineEmits<{
  (e: 'copyPermissionDetail', value: string | null | undefined, key: 'executable' | 'bundle' | 'bundleId'): void;
  (e: 'openAccessibilitySettings'): void;
  (e: 'probePermission'): void;
}>();

const accessibilityRecoveryRef = ref<HTMLElement | null>(null);

defineExpose({
  accessibilityRecoveryRef,
});
</script>

<template>
  <div
    v-if="showAccessibilityRecovery"
    ref="accessibilityRecoveryRef"
    class="cyber-panel flex flex-col gap-3 px-4 py-3 border-rose-500/40"
  >
    <div class="flex flex-col gap-3 xl:flex-row xl:items-start">
      <Icon icon="lucide:shield-alert" class="text-base text-rose-400 shrink-0 mt-0.5" />
      <div class="flex-1 flex flex-col gap-2 min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-[11px] font-bold text-rose-300 uppercase tracking-wider">
            {{ t('recovery.title') }}
          </span>
          <button
            type="button"
            class="font-mono text-[9px] text-cyan-300 hover:text-cyan-200 cursor-pointer underline decoration-dotted"
            :title="inputPermissionDiagnostics?.bundleIdentifier || t('recovery.bundleIdentifierFallback')"
            @click="
              emit('copyPermissionDetail', inputPermissionDiagnostics?.bundleIdentifier, 'bundleId')
            "
          >
            {{
              permissionCopyHint === 'bundleId'
                ? t('recovery.copiedBundleId')
                : shortBundleIdentifier || t('recovery.bundleIdUnknown')
            }}
          </button>
        </div>
        <p class="text-[10px] text-slate-400 leading-relaxed">
          {{ inputPermissionActionText }} {{ t('recovery.resetSteps') }}
        </p>
        <div class="grid gap-2 xl:grid-cols-2">
          <button
            type="button"
            class="cyber-inset flex min-w-0 items-center justify-between gap-2 p-2 text-left cursor-pointer"
            :disabled="!inputPermissionDiagnostics?.executablePath"
            @click="
              emit('copyPermissionDetail', inputPermissionDiagnostics?.executablePath, 'executable')
            "
            :title="t('recovery.copyExecutablePath')"
          >
            <span class="min-w-0">
              <span class="block text-[9px] uppercase tracking-widest font-bold text-slate-500">
                executablePath
              </span>
              <span class="block font-mono text-[9px] text-slate-300 truncate select-text">
                {{ inputPermissionDiagnostics?.executablePath || t('recovery.couldNotResolve') }}
              </span>
            </span>
            <Icon
              :icon="permissionCopyHint === 'executable' ? 'lucide:check' : 'lucide:copy'"
              class="text-xs text-cyan-400 shrink-0"
            />
          </button>
          <button
            type="button"
            class="cyber-inset flex min-w-0 items-center justify-between gap-2 p-2 text-left cursor-pointer"
            :disabled="!inputPermissionDiagnostics?.appBundlePath"
            @click="emit('copyPermissionDetail', inputPermissionDiagnostics?.appBundlePath, 'bundle')"
            :title="t('recovery.copyAppBundlePath')"
          >
            <span class="min-w-0">
              <span class="block text-[9px] uppercase tracking-widest font-bold text-slate-500">
                appBundlePath
              </span>
              <span class="block font-mono text-[9px] text-slate-300 truncate select-text">
                {{
                  inputPermissionDiagnostics?.appBundlePath ||
                  (inputPermissionDiagnostics?.isPackagedApp
                    ? t('recovery.couldNotResolve')
                    : t('recovery.devBinary'))
                }}
              </span>
            </span>
            <Icon
              :icon="permissionCopyHint === 'bundle' ? 'lucide:check' : 'lucide:copy'"
              class="text-xs text-cyan-400 shrink-0"
            />
          </button>
        </div>
      </div>
      <div class="flex flex-wrap gap-2 xl:justify-end">
        <button
          class="cyber-action-btn font-bold cursor-pointer text-[10px] uppercase tracking-wider px-3 py-1.5 flex items-center gap-1.5"
          @click="emit('openAccessibilitySettings')"
        >
          <Icon icon="lucide:external-link" class="text-xs" />
          <span>{{ t('recovery.openSettings') }}</span>
        </button>
        <button
          class="cyber-action-btn font-bold cursor-pointer text-[10px] uppercase tracking-wider px-3 py-1.5 flex items-center gap-1.5"
          @click="emit('probePermission')"
          :title="t('recovery.recheckTitle')"
        >
          <Icon icon="lucide:refresh-cw" class="text-xs" />
          <span>{{ t('recovery.recheck') }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
