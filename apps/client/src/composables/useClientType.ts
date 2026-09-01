import { onMounted, ref } from 'vue'

export type ClientType = 'everyone' | 'username' | 'discord'

const clientType = ref<ClientType>('everyone')

export function useClientType(): {
  clientType: typeof clientType
  setClient: (type: ClientType) => void
} {
  const setClient = (type: ClientType): void => {
    clientType.value = type

    save()
  }

  const save = (): void => {
    localStorage.setItem('clientType', clientType.value)
  }

  const load = (): void => {
    clientType.value = (localStorage.getItem('clientType') as ClientType) || 'everyone'
  }

  onMounted(() => load())

  return { clientType, setClient }
}
