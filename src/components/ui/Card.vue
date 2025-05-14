<script setup>
defineProps({
  variant: {
    type: String,
    default: 'default',
    validator: (value) => ['default', 'hoverable', 'bordered', 'flat'].includes(value)
  },
  padding: {
    type: String,
    default: 'md',
    validator: (value) => ['none', 'sm', 'md', 'lg', 'xl'].includes(value)
  }
})
</script>

<template>
  <div
    class="bg-white rounded-xl transition-all duration-200"
    :class="[
      // Variants
      variant === 'default' && 'shadow-sm',
      variant === 'hoverable' && 'shadow-sm hover:shadow-md hover:-translate-y-1',
      variant === 'bordered' && 'border border-gray-200',
      variant === 'flat' && 'bg-gray-50',
      
      // Padding
      padding === 'none' && 'p-0',
      padding === 'sm' && 'p-3',
      padding === 'md' && 'p-4',
      padding === 'lg' && 'p-6',
      padding === 'xl' && 'p-8',
    ]"
  >
    <slot></slot>
  </div>
</template>

<style scoped>
.hover\:shadow-md:hover {
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}
</style>